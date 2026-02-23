use z3::ast::{Ast, Int, Bool, Array, Dynamic, Float};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::{Atom, QuantifierType, Expr, Op, parse_expression, RefinedType, StructDef};
use std::fs;
use std::path::Path;
use std::fmt;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// --- エラー型の定義 ---
#[derive(Debug)]
pub enum MumeiError {
    VerificationError(String),
    CodegenError(String),
    TypeError(String),
}

impl fmt::Display for MumeiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MumeiError::VerificationError(msg) => write!(f, "Verification Error: {}", msg),
            MumeiError::CodegenError(msg) => write!(f, "Codegen Error: {}", msg),
            MumeiError::TypeError(msg) => write!(f, "Type Error: {}", msg),
        }
    }
}

impl From<String> for MumeiError {
    fn from(s: String) -> Self {
        MumeiError::VerificationError(s)
    }
}

impl From<&str> for MumeiError {
    fn from(s: &str) -> Self {
        MumeiError::VerificationError(s.to_string())
    }
}

pub type MumeiResult<T> = Result<T, MumeiError>;
type Env<'a> = HashMap<String, Dynamic<'a>>;
type DynResult<'a> = MumeiResult<Dynamic<'a>>;

/// 検証時に共有するコンテキスト（ctx, arr を束ねて引数を削減）
struct VCtx<'a> {
    ctx: &'a Context,
    arr: &'a Array<'a>,
}

// --- 型環境のグローバル管理 ---
static TYPE_ENV: Lazy<Mutex<HashMap<String, RefinedType>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

static STRUCT_ENV: Lazy<Mutex<HashMap<String, StructDef>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

pub fn register_type(refined_type: &RefinedType) -> MumeiResult<()> {
    let mut env = TYPE_ENV.lock().map_err(|_| MumeiError::TypeError("Failed to lock TYPE_ENV".into()))?;
    env.insert(refined_type.name.clone(), refined_type.clone());
    Ok(())
}

pub fn register_struct(struct_def: &StructDef) -> MumeiResult<()> {
    let mut env = STRUCT_ENV.lock().map_err(|_| MumeiError::TypeError("Failed to lock STRUCT_ENV".into()))?;
    env.insert(struct_def.name.clone(), struct_def.clone());
    Ok(())
}

pub fn get_struct_def(name: &str) -> Option<StructDef> {
    STRUCT_ENV.lock().ok().and_then(|env| env.get(name).cloned())
}

/// 精緻型名からベース型名を解決する（例: "Nat" -> "i64", "Pos" -> "f64"）
/// 未登録の型名はそのまま返す
pub fn resolve_base_type(type_name: &str) -> String {
    if let Ok(env) = TYPE_ENV.lock() {
        if let Some(refined) = env.get(type_name) {
            return refined._base_type.clone();
        }
    }
    type_name.to_string()
}

pub fn verify(atom: &Atom, output_dir: &Path) -> MumeiResult<()> {
    let mut cfg = Config::new();
    cfg.set_timeout_msec(10000);
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let int_sort = z3::Sort::int(&ctx);
    let arr = Array::new_const(&ctx, "arr", &int_sort, &int_sort);
    let vc = VCtx { ctx: &ctx, arr: &arr };

    let mut env: Env = HashMap::new();

    // 1. 量子化制約の処理
    for q in &atom.forall_constraints {
        let i = Int::new_const(&ctx, q.var.as_str());
        let start = Int::from_i64(&ctx, q.start.parse::<i64>().unwrap_or(0));
        let end = if let Ok(val) = q.end.parse::<i64>() {
            Int::from_i64(&ctx, val)
        } else {
            Int::new_const(&ctx, q.end.as_str())
        };

        let range_cond = Bool::and(&ctx, &[&i.ge(&start), &i.lt(&end)]);
        let expr_ast = parse_expression(&q.condition);
        let condition_z3 = expr_to_z3(&vc, &expr_ast, &mut env, None)?
            .as_bool().ok_or(MumeiError::VerificationError("Condition must be boolean".into()))?;

        let quantifier_expr = match q.q_type {
            QuantifierType::ForAll => z3::ast::forall_const(&ctx, &[&i], &[], &range_cond.implies(&condition_z3)),
            QuantifierType::Exists => z3::ast::exists_const(&ctx, &[&i], &[], &Bool::and(&ctx, &[&range_cond, &condition_z3])),
        };
        solver.assert(&quantifier_expr);
    }

    // 2. 引数（params）に対する精緻型制約の自動適用
    {
        let type_defs = TYPE_ENV.lock().map_err(|_| MumeiError::TypeError("Failed to lock TYPE_ENV".into()))?;
        for param in &atom.params {
            if let Some(type_name) = &param.type_name {
                if let Some(refined) = type_defs.get(type_name) {
                    apply_refinement_constraint(&vc, &solver, &param.name, refined, &mut env)?;
                }
            }
        }
    }

    // 2b. 引数（params）に対する構造体フィールド制約の自動適用
    {
        let struct_defs = STRUCT_ENV.lock().map_err(|_| MumeiError::TypeError("Failed to lock STRUCT_ENV".into()))?;
        for param in &atom.params {
            if let Some(type_name) = &param.type_name {
                if let Some(sdef) = struct_defs.get(type_name) {
                    // 構造体の各フィールドをシンボリック変数として env に登録し、制約を適用
                    for field in &sdef.fields {
                        let field_var_name = format!("{}_{}", param.name, field.name);
                        let base = resolve_base_type(&field.type_name);
                        let field_z3: Dynamic = match base.as_str() {
                            "f64" => Float::new_const(&ctx, field_var_name.as_str(), 11, 53).into(),
                            _ => Int::new_const(&ctx, field_var_name.as_str()).into(),
                        };
                        env.insert(field_var_name.clone(), field_z3.clone());
                        // qualified name も登録
                        let qualified = format!("__struct_{}_{}", param.name, field.name);
                        env.insert(qualified, field_z3.clone());

                        // フィールド制約を solver に assert
                        if let Some(constraint_raw) = &field.constraint {
                            let mut local_env = env.clone();
                            local_env.insert("v".to_string(), field_z3);
                            let constraint_ast = parse_expression(constraint_raw);
                            let constraint_z3 = expr_to_z3(&vc, &constraint_ast, &mut local_env, None)?;
                            if let Some(constraint_bool) = constraint_z3.as_bool() {
                                solver.assert(&constraint_bool);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2c. 全パラメータに対して配列長シンボルを事前生成
    for param in &atom.params {
        let len_name = format!("len_{}", param.name);
        if !env.contains_key(&len_name) {
            let len_var = Int::new_const(&ctx, len_name.as_str());
            solver.assert(&len_var.ge(&Int::from_i64(&ctx, 0)));
            env.insert(len_name, len_var.into());
        }
    }

    // 3. 前提条件 (requires)
    if atom.requires.trim() != "true" {
        let req_ast = parse_expression(&atom.requires);
        let req_z3 = expr_to_z3(&vc, &req_ast, &mut env, None)?;
        if let Some(req_bool) = req_z3.as_bool() {
            solver.assert(&req_bool);
        }
    }

    // 4. ボディの検証
    let body_ast = parse_expression(&atom.body_expr);
    let body_result = expr_to_z3(&vc, &body_ast, &mut env, Some(&solver))?;

    // 5. 事後条件 (ensures)
    if atom.ensures.trim() != "true" {
        env.insert("result".to_string(), body_result);
        let ens_ast = parse_expression(&atom.ensures);
        let ens_z3 = expr_to_z3(&vc, &ens_ast, &mut env, None)?;
        if let Some(ens_bool) = ens_z3.as_bool() {
            solver.push();
            solver.assert(&ens_bool.not());
            if solver.check() == SatResult::Sat {
                solver.pop(1);
                save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Postcondition violated.");
                return Err(MumeiError::VerificationError("Postcondition (ensures) is not satisfied.".into()));
            }
            solver.pop(1);
        }
        env.remove("result");
    }

    if solver.check() == SatResult::Unsat {
        save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Logic contradiction.");
        return Err(MumeiError::VerificationError("Contradiction found.".into()));
    }

    save_visualizer_report(output_dir, "success", &atom.name, "N/A", "N/A", "Verified safe.");
    Ok(())
}

fn apply_refinement_constraint<'a>(
    vc: &VCtx<'a>,
    solver: &Solver<'a>,
    var_name: &str,
    refined: &RefinedType,
    global_env: &mut Env<'a>
) -> MumeiResult<()> {
    let ctx = vc.ctx;
    // Type System 2.0: ベース型に基づいて変数を生成
    let var_z3: Dynamic = match refined._base_type.as_str() {
        "f64" => Float::new_const(ctx, var_name, 11, 53).into(),
        "u64" => {
            let v = Int::new_const(ctx, var_name);
            solver.assert(&v.ge(&Int::from_i64(ctx, 0)));
            v.into()
        },
        _ => Int::new_const(ctx, var_name).into(),
    };

    global_env.insert(var_name.to_string(), var_z3.clone());

    let mut local_env = global_env.clone();
    local_env.insert(refined.operand.clone(), var_z3);

    let predicate_ast = parse_expression(&refined.predicate_raw);
    let predicate_z3 = expr_to_z3(vc, &predicate_ast, &mut local_env, None)?
        .as_bool().ok_or(MumeiError::TypeError(format!("Predicate for {} must be boolean", refined.name)))?;

    solver.assert(&predicate_z3);
    Ok(())
}

fn expr_to_z3<'a>(
    vc: &VCtx<'a>,
    expr: &Expr,
    env: &mut Env<'a>,
    solver_opt: Option<&Solver<'a>>
) -> DynResult<'a> {
    let ctx = vc.ctx;
    let arr = vc.arr;
    match expr {
        Expr::Number(n) => Ok(Int::from_i64(ctx, *n).into()),
        Expr::Float(f) => Ok(Float::from_f64(ctx, *f).into()),
        Expr::Variable(name) => {
            Ok(env.get(name).cloned().unwrap_or_else(|| Int::new_const(ctx, name.as_str()).into()))
        },
        Expr::Call(name, args) => {
            match name.as_str() {
                "len" => {
                    // len(arr_name) → 配列名に紐づくシンボリック長を返す
                    // len_<name> >= 0 の制約を自動付与
                    let arr_name = if !args.is_empty() {
                        if let Expr::Variable(name) = &args[0] { name.clone() } else { "arr".to_string() }
                    } else { "arr".to_string() };
                    let len_name = format!("len_{}", arr_name);
                    let len_var = Int::new_const(ctx, len_name.as_str());
                    if let Some(solver) = solver_opt {
                        solver.assert(&len_var.ge(&Int::from_i64(ctx, 0)));
                    }
                    env.insert(len_name, len_var.clone().into());
                    Ok(len_var.into())
                },
                "sqrt" => {
                    // Z3 0.12 の Float には sqrt メソッドがないため、
                    // シンボリック変数として扱い、sqrt(x) >= 0 の制約を付与
                    let _val = expr_to_z3(vc, &args[0], env, solver_opt)?;
                    let result = Float::new_const(ctx, "sqrt_result", 11, 53);
                    if let Some(solver) = solver_opt {
                        let zero = Float::from_f64(ctx, 0.0);
                        solver.assert(&result.ge(&zero));
                    }
                    Ok(result.into())
                },
                "cast_to_int" => {
                    // Z3 0.12 では Float->Int 直接変換がないため、シンボリック整数を返す
                    let _val = expr_to_z3(vc, &args[0], env, solver_opt)?;
                    Ok(Int::new_const(ctx, "cast_result").into())
                }
                _ => Err(MumeiError::VerificationError(format!("Unknown function: {}", name))),
            }
        },
        Expr::ArrayAccess(name, index_expr) => {
            let idx = expr_to_z3(vc, index_expr, env, solver_opt)?
                .as_int().ok_or(MumeiError::TypeError("Index must be integer".into()))?;

            // 配列名に紐づく長さシンボルを使った境界チェック
            if let Some(solver) = solver_opt {
                let len_name = format!("len_{}", name);
                let len = if let Some(existing) = env.get(&len_name) {
                    existing.as_int().unwrap_or(Int::new_const(ctx, len_name.as_str()))
                } else {
                    let l = Int::new_const(ctx, len_name.as_str());
                    solver.assert(&l.ge(&Int::from_i64(ctx, 0)));
                    env.insert(len_name.clone(), l.clone().into());
                    l
                };
                let safe = Bool::and(ctx, &[&idx.ge(&Int::from_i64(ctx, 0)), &idx.lt(&len)]);
                solver.push();
                solver.assert(&safe.not());
                if solver.check() == SatResult::Sat {
                    solver.pop(1);
                    return Err(MumeiError::VerificationError(format!("Potential Out-of-Bounds on '{}' (index may be < 0 or >= len_{})", name, name)));
                }
                solver.pop(1);
            }
            Ok(arr.select(&idx).into())
        },
        Expr::BinaryOp(left, op, right) => {
            let l = expr_to_z3(vc, left, env, solver_opt)?;
            let r = expr_to_z3(vc, right, env, solver_opt)?;

            // 浮動小数点か整数かで Z3 の AST メソッドを使い分ける
            if l.as_float().is_some() || r.as_float().is_some() {
                // 浮動小数点の場合、比較演算のみサポート（z3 0.12 の Float 算術は丸めモード API が複雑なため）
                // 算術演算はシンボリック結果として返す
                let lf = l.as_float().unwrap_or(Float::from_f64(ctx, 0.0));
                let rf = r.as_float().unwrap_or(Float::from_f64(ctx, 0.0));
                match op {
                    Op::Gt  => Ok(lf.gt(&rf).into()),
                    Op::Lt  => Ok(lf.lt(&rf).into()),
                    Op::Ge  => Ok(lf.ge(&rf).into()),
                    Op::Le  => Ok(lf.le(&rf).into()),
                    Op::Eq  => Ok(lf._eq(&rf).into()),
                    Op::Neq => Ok(lf._eq(&rf).not().into()),
                    Op::Add | Op::Sub | Op::Mul | Op::Div => {
                        // シンボリック Float + 符号伝播制約
                        // (z3 crate 0.12 は内部フィールドが非公開のため z3-sys 直接呼び出し不可)
                        static FLOAT_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                        let id = FLOAT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let result = Float::new_const(ctx, format!("float_arith_{}", id), 11, 53);
                        let zero = Float::from_f64(ctx, 0.0);
                        if let Some(solver) = solver_opt {
                            match op {
                                Op::Mul => {
                                    let both_pos = Bool::and(ctx, &[&lf.gt(&zero), &rf.gt(&zero)]);
                                    solver.assert(&both_pos.implies(&result.gt(&zero)));
                                    let both_neg = Bool::and(ctx, &[&lf.lt(&zero), &rf.lt(&zero)]);
                                    solver.assert(&both_neg.implies(&result.gt(&zero)));
                                },
                                Op::Add => {
                                    let both_pos = Bool::and(ctx, &[&lf.gt(&zero), &rf.ge(&zero)]);
                                    solver.assert(&both_pos.implies(&result.gt(&zero)));
                                    let both_pos2 = Bool::and(ctx, &[&lf.ge(&zero), &rf.gt(&zero)]);
                                    solver.assert(&both_pos2.implies(&result.gt(&zero)));
                                },
                                Op::Sub => {
                                    let a_gt_b = Bool::and(ctx, &[&lf.gt(&rf), &rf.ge(&zero)]);
                                    solver.assert(&a_gt_b.implies(&result.ge(&zero)));
                                },
                                Op::Div => {
                                    let both_pos = Bool::and(ctx, &[&lf.gt(&zero), &rf.gt(&zero)]);
                                    solver.assert(&both_pos.implies(&result.gt(&zero)));
                                },
                                _ => {}
                            }
                        }
                        Ok(result.into())
                    },
                    _ => Err("Invalid float op".into()),
                }
            } else {
                // Boolean 演算子は as_int() の前に処理する（オペランドが Bool のため）
                match op {
                    Op::And => {
                        let lb = l.as_bool().ok_or("Expected bool for &&")?;
                        let rb = r.as_bool().ok_or("Expected bool for &&")?;
                        return Ok(Bool::and(ctx, &[&lb, &rb]).into());
                    },
                    Op::Or => {
                        let lb = l.as_bool().ok_or("Expected bool for ||")?;
                        let rb = r.as_bool().ok_or("Expected bool for ||")?;
                        return Ok(Bool::or(ctx, &[&lb, &rb]).into());
                    },
                    Op::Implies => {
                        let lb = l.as_bool().ok_or("Expected bool for =>")?;
                        let rb = r.as_bool().ok_or("Expected bool for =>")?;
                        return Ok(lb.implies(&rb).into());
                    },
                    _ => {}
                }
                let li = l.as_int().ok_or("Expected int")?;
                let ri = r.as_int().ok_or("Expected int")?;
                match op {
                    Op::Add => Ok((&li + &ri).into()),
                    Op::Sub => Ok((&li - &ri).into()),
                    Op::Mul => Ok((&li * &ri).into()),
                    Op::Div => {
                        if let Some(solver) = solver_opt {
                            solver.push();
                            solver.assert(&ri._eq(&Int::from_i64(ctx, 0)));
                            if solver.check() == SatResult::Sat {
                                solver.pop(1);
                                return Err(MumeiError::VerificationError("Potential division by zero.".into()));
                            }
                            solver.pop(1);
                        }
                        Ok((&li / &ri).into())
                    },
                    Op::Gt  => Ok(li.gt(&ri).into()),
                    Op::Lt  => Ok(li.lt(&ri).into()),
                    Op::Ge  => Ok(li.ge(&ri).into()),
                    Op::Le  => Ok(li.le(&ri).into()),
                    Op::Eq  => Ok(li._eq(&ri).into()),
                    Op::Neq => Ok(li._eq(&ri).not().into()),
                    _ => Err(MumeiError::VerificationError(format!("Unsupported int operator {:?}", op))),
                }
            }
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let c = expr_to_z3(vc, cond, env, solver_opt)?
                .as_bool().ok_or(MumeiError::TypeError("If condition must be boolean".into()))?;
            let t = expr_to_z3(vc, then_branch, env, solver_opt)?;
            let e = expr_to_z3(vc, else_branch, env, solver_opt)?;
            Ok(c.ite(&t, &e))
        },
        Expr::Let { var, value } => {
            // Block 内の逐次実行では変数を env に残す（スコープ管理は Block 側で行う）
            let val = expr_to_z3(vc, value, env, solver_opt)?;
            env.insert(var.clone(), val.clone());
            Ok(val)
        },
        Expr::Assign { var, value } => {
            let val = expr_to_z3(vc, value, env, solver_opt)?;
            env.insert(var.clone(), val.clone());
            Ok(val)
        },
        Expr::Block(stmts) => {
            let mut last = Int::from_i64(ctx, 0).into();
            for stmt in stmts { last = expr_to_z3(vc, stmt, env, solver_opt)?; }
            Ok(last)
        },
        Expr::While { cond, invariant, decreases, body } => {
            // Loop Invariant 検証ロジック
            if let Some(solver) = solver_opt {
                let inv = expr_to_z3(vc, invariant, env, None)?
                    .as_bool().ok_or(MumeiError::TypeError("Invariant must be boolean".into()))?;

                // Base case: 現在の env（let で初期化済み）で invariant が成立するか
                solver.push();
                solver.assert(&inv.not());
                if solver.check() == SatResult::Sat {
                    solver.pop(1);
                    return Err(MumeiError::VerificationError("Invariant fails initially".into()));
                }
                solver.pop(1);

                // Inductive step: invariant && cond のもとで body 実行後も invariant が保たれるか
                let c = expr_to_z3(vc, cond, env, None)?
                    .as_bool().ok_or(MumeiError::TypeError("While condition must be boolean".into()))?;

                // Invariant preservation: invariant && cond のもとで body 実行後も invariant が保たれるか
                // env のスナップショットを保存し、各チェックを独立に行う
                {
                    let env_snapshot = env.clone();
                    solver.push();
                    solver.assert(&inv);
                    solver.assert(&c);
                    expr_to_z3(vc, body, env, Some(solver))?;

                    let inv_after = expr_to_z3(vc, invariant, env, None)?
                        .as_bool().ok_or(MumeiError::TypeError("Invariant must be boolean".into()))?;

                    solver.assert(&inv_after.not());
                    if solver.check() == SatResult::Sat {
                        solver.pop(1);
                        return Err(MumeiError::VerificationError("Invariant not preserved".into()));
                    }
                    solver.pop(1);
                    *env = env_snapshot; // env を復元
                }

                // Termination Check: decreases 句が指定されている場合、停止性を検証
                if let Some(dec_expr) = decreases {
                    let env_snapshot = env.clone();

                    // V_before: ループ本体実行前の減少式の値
                    let v_before = expr_to_z3(vc, dec_expr, env, None)?
                        .as_int().ok_or(MumeiError::TypeError("decreases expression must be integer".into()))?;

                    // A. 下界の証明: invariant && cond => V >= 0
                    solver.push();
                    solver.assert(&inv);
                    solver.assert(&c);
                    solver.assert(&v_before.lt(&Int::from_i64(ctx, 0)));
                    if solver.check() == SatResult::Sat {
                        solver.pop(1);
                        return Err(MumeiError::VerificationError(
                            "Termination check failed: decreases expression may be negative".into()
                        ));
                    }
                    solver.pop(1);

                    // B. 厳密な減少の証明: body 実行後に V' < V
                    solver.push();
                    solver.assert(&inv);
                    solver.assert(&c);
                    expr_to_z3(vc, body, env, Some(solver))?;

                    let v_after = expr_to_z3(vc, dec_expr, env, None)?
                        .as_int().ok_or(MumeiError::TypeError("decreases expression must be integer".into()))?;

                    solver.assert(&v_after.ge(&v_before));
                    if solver.check() == SatResult::Sat {
                        solver.pop(1);
                        *env = env_snapshot;
                        return Err(MumeiError::VerificationError(
                            "Termination check failed: decreases expression does not strictly decrease".into()
                        ));
                    }
                    solver.pop(1);
                    *env = env_snapshot; // env を復元
                }
            }

            let inv = expr_to_z3(vc, invariant, env, None)?
                .as_bool().ok_or(MumeiError::TypeError("Invariant must be boolean".into()))?;
            let c_not = expr_to_z3(vc, cond, env, None)?
                .as_bool().ok_or(MumeiError::TypeError("While condition must be boolean".into()))?
                .not();
            Ok(Bool::and(ctx, &[&inv, &c_not]).into())
        },
        Expr::StructInit { type_name, fields } => {
            // 構造体の各フィールドを検証し、env に登録
            // フィールドに精緻型制約がある場合は solver で検証する
            let mut last: Dynamic = Int::from_i64(ctx, 0).into();
            for (field_name, field_expr) in fields {
                let val = expr_to_z3(vc, field_expr, env, solver_opt)?;
                let qualified_name = format!("__struct_{}_{}", type_name, field_name);
                env.insert(qualified_name, val.clone());
                last = val.clone();

                // フィールド制約の検証: 構造体定義から constraint を取得
                if let Some(sdef) = get_struct_def(type_name) {
                    if let Some(sfield) = sdef.fields.iter().find(|f| f.name == *field_name) {
                        if let Some(constraint_raw) = &sfield.constraint {
                            // constraint 内の "v" をフィールド値に置き換えて検証
                            let mut local_env = env.clone();
                            local_env.insert("v".to_string(), val.clone());
                            let constraint_ast = parse_expression(constraint_raw);
                            let constraint_z3 = expr_to_z3(vc, &constraint_ast, &mut local_env, None)?;
                            if let Some(constraint_bool) = constraint_z3.as_bool() {
                                if let Some(solver) = solver_opt {
                                    solver.push();
                                    solver.assert(&constraint_bool.not());
                                    if solver.check() == SatResult::Sat {
                                        solver.pop(1);
                                        return Err(MumeiError::VerificationError(
                                            format!("Struct '{}' field '{}' constraint violated: {}", type_name, field_name, constraint_raw)
                                        ));
                                    }
                                    solver.pop(1);
                                }
                            }
                        }
                    }
                }
            }
            Ok(last)
        },
        Expr::FieldAccess(expr, field_name) => {
            // v.x → env から __struct_TypeName_x を探す、または v_x として探す
            if let Expr::Variable(var_name) = expr.as_ref() {
                // まず qualified name で探す
                let candidates = [
                    format!("__struct_{}_{}", var_name, field_name),
                    format!("{}_{}", var_name, field_name),
                ];
                for candidate in &candidates {
                    if let Some(val) = env.get(candidate) {
                        return Ok(val.clone());
                    }
                }
                // 見つからなければシンボリック変数を生成
                let sym_name = format!("{}_{}", var_name, field_name);
                let sym = Int::new_const(ctx, sym_name.as_str());
                env.insert(sym_name, sym.clone().into());
                Ok(sym.into())
            } else {
                // ネストされたフィールドアクセスはシンボリック変数で近似
                let _base = expr_to_z3(vc, expr, env, solver_opt)?;
                let sym = Int::new_const(ctx, format!("field_{}", field_name));
                Ok(sym.into())
            }
        },
    }
}

fn save_visualizer_report(output_dir: &Path, status: &str, name: &str, a: &str, b: &str, reason: &str) {
    let report = json!({ "status": status, "atom": name, "input_a": a, "input_b": b, "reason": reason });
    let _ = fs::create_dir_all(output_dir);
    let _ = fs::write(output_dir.join("report.json"), report.to_string());
}
