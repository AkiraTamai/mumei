use z3::ast::{Ast, Int, Bool, Array, Dynamic, Float};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::{Atom, QuantifierType, Expr, Op, parse_expression, RefinedType};
use std::fs;
use std::path::Path;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// --- 型環境のグローバル管理 ---
static TYPE_ENV: Lazy<Mutex<HashMap<String, RefinedType>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

pub fn register_type(refined_type: &RefinedType) -> Result<(), String> {
    let mut env = TYPE_ENV.lock().map_err(|_| "Failed to lock TYPE_ENV")?;
    env.insert(refined_type.name.clone(), refined_type.clone());
    Ok(())
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

pub fn verify(atom: &Atom, output_dir: &Path) -> Result<(), String> {
    let mut cfg = Config::new();
    cfg.set_timeout_msec(10000);
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let int_sort = z3::Sort::int(&ctx);
    // 配列のモデル（簡易版: i64 -> i64）。将来的に型ごとに Array Sort を分ける拡張が可能
    let arr = Array::new_const(&ctx, "arr", &int_sort, &int_sort);

    let mut env: HashMap<String, Dynamic> = HashMap::new();

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
        let condition_z3 = expr_to_z3(&ctx, &arr, &expr_ast, &mut env, None)?
            .as_bool().ok_or("Condition must be boolean")?;

        let quantifier_expr = match q.q_type {
            QuantifierType::ForAll => z3::ast::forall_const(&ctx, &[&i], &[], &range_cond.implies(&condition_z3)),
            QuantifierType::Exists => z3::ast::exists_const(&ctx, &[&i], &[], &Bool::and(&ctx, &[&range_cond, &condition_z3])),
        };
        solver.assert(&quantifier_expr);
    }

    // 2. 引数（params）に対する精緻型制約の自動適用
    {
        let type_defs = TYPE_ENV.lock().map_err(|_| "Failed to lock TYPE_ENV")?;
        for param in &atom.params {
            if let Some(type_name) = &param.type_name {
                if let Some(refined) = type_defs.get(type_name) {
                    apply_refinement_constraint(&ctx, &arr, &solver, &param.name, refined, &mut env)?;
                }
            }
        }
    }

    // 3. 前提条件 (requires)
    if atom.requires.trim() != "true" {
        let req_ast = parse_expression(&atom.requires);
        let req_z3 = expr_to_z3(&ctx, &arr, &req_ast, &mut env, None)?;
        if let Some(req_bool) = req_z3.as_bool() {
            solver.assert(&req_bool);
        }
    }

    // 4. ボディの検証
    let body_ast = parse_expression(&atom.body_expr);
    let body_result = expr_to_z3(&ctx, &arr, &body_ast, &mut env, Some(&solver))?;

    // 5. 事後条件 (ensures)
    if atom.ensures.trim() != "true" {
        env.insert("result".to_string(), body_result);
        let ens_ast = parse_expression(&atom.ensures);
        let ens_z3 = expr_to_z3(&ctx, &arr, &ens_ast, &mut env, None)?;
        if let Some(ens_bool) = ens_z3.as_bool() {
            solver.push();
            solver.assert(&ens_bool.not());
            if solver.check() == SatResult::Sat {
                solver.pop(1);
                save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Postcondition violated.");
                return Err("Verification Error: Postcondition (ensures) is not satisfied.".into());
            }
            solver.pop(1);
        }
        env.remove("result");
    }

    if solver.check() == SatResult::Unsat {
        save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Logic contradiction.");
        return Err("Verification failed: Contradiction found.".into());
    }

    save_visualizer_report(output_dir, "success", &atom.name, "N/A", "N/A", "Verified safe.");
    Ok(())
}

fn apply_refinement_constraint<'a>(
    ctx: &'a Context,
    arr: &Array<'a>,
    solver: &Solver<'a>,
    var_name: &str,
    refined: &RefinedType,
    global_env: &mut HashMap<String, Dynamic<'a>>
) -> Result<(), String> {
    // Type System 2.0: ベース型に基づいて変数を生成
    let var_z3: Dynamic = match refined._base_type.as_str() {
        "f64" => Float::new_const(ctx, var_name, 11, 53).into(),
        "u64" => {
            let v = Int::new_const(ctx, var_name);
            solver.assert(&v.ge(&Int::from_i64(ctx, 0))); // u64 の基本制約: v >= 0
            v.into()
        },
        _ => Int::new_const(ctx, var_name).into(),
    };

    global_env.insert(var_name.to_string(), var_z3.clone());

    let mut local_env = global_env.clone();
    local_env.insert(refined.operand.clone(), var_z3);

    let predicate_ast = parse_expression(&refined.predicate_raw);
    let predicate_z3 = expr_to_z3(ctx, arr, &predicate_ast, &mut local_env, None)?
        .as_bool().ok_or(format!("Predicate for {} must be boolean", refined.name))?;

    solver.assert(&predicate_z3);
    Ok(())
}

fn expr_to_z3<'a>(
    ctx: &'a Context,
    arr: &Array<'a>,
    expr: &Expr,
    env: &mut HashMap<String, Dynamic<'a>>,
    solver_opt: Option<&Solver<'a>>
) -> Result<Dynamic<'a>, String> {
    match expr {
        Expr::Number(n) => Ok(Int::from_i64(ctx, *n).into()),
        Expr::Float(f) => Ok(Float::from_f64(ctx, *f).into()),
        Expr::Variable(name) => {
            Ok(env.get(name).cloned().unwrap_or_else(|| Int::new_const(ctx, name.as_str()).into()))
        },
        Expr::Call(name, args) => {
            match name.as_str() {
                "len" => Ok(Int::new_const(ctx, "arr_len").into()),
                "sqrt" => {
                    // Z3 0.12 の Float には sqrt メソッドがないため、
                    // シンボリック変数として扱い、sqrt(x) >= 0 の制約を付与
                    let _val = expr_to_z3(ctx, arr, &args[0], env, solver_opt)?;
                    let result = Float::new_const(ctx, "sqrt_result", 11, 53);
                    if let Some(solver) = solver_opt {
                        let zero = Float::from_f64(ctx, 0.0);
                        solver.assert(&result.ge(&zero));
                    }
                    Ok(result.into())
                },
                "cast_to_int" => {
                    // Z3 0.12 では Float->Int 直接変換がないため、シンボリック整数を返す
                    let _val = expr_to_z3(ctx, arr, &args[0], env, solver_opt)?;
                    Ok(Int::new_const(ctx, "cast_result").into())
                }
                _ => Err(format!("Unknown function: {}", name)),
            }
        },
        Expr::ArrayAccess(name, index_expr) => {
            let idx = expr_to_z3(ctx, arr, index_expr, env, solver_opt)?
                .as_int().ok_or("Index must be integer")?;

            // Standard Library: 境界チェックの自動挿入
            if let Some(solver) = solver_opt {
                let len = Int::new_const(ctx, "arr_len");
                let safe = Bool::and(ctx, &[&idx.ge(&Int::from_i64(ctx, 0)), &idx.lt(&len)]);
                solver.push();
                solver.assert(&safe.not());
                if solver.check() == SatResult::Sat {
                    solver.pop(1);
                    return Err(format!("Verification Error: Potential Out-of-Bounds on '{}'", name));
                }
                solver.pop(1);
            }
            Ok(arr.select(&idx).into())
        },
        Expr::BinaryOp(left, op, right) => {
            let l = expr_to_z3(ctx, arr, left, env, solver_opt)?;
            let r = expr_to_z3(ctx, arr, right, env, solver_opt)?;

            // 浮動小数点か整数かで Z3 の AST メソッドを使い分ける
            if l.as_float().is_some() || r.as_float().is_some() {
                let lf = l.as_float().unwrap_or(Float::from_f64(ctx, 0.0));
                let rf = r.as_float().unwrap_or(Float::from_f64(ctx, 0.0));
                let rm = Float::round_nearest_ties_to_even(ctx);
                match op {
                    Op::Add => Ok(lf.add(&rm, &rf).into()),
                    Op::Sub => Ok(lf.sub(&rm, &rf).into()),
                    Op::Mul => Ok(lf.mul(&rm, &rf).into()),
                    Op::Div => Ok(lf.div(&rm, &rf).into()),
                    Op::Gt  => Ok(lf.gt(&rf).into()),
                    Op::Lt  => Ok(lf.lt(&rf).into()),
                    Op::Ge  => Ok(lf.ge(&rf).into()),
                    Op::Le  => Ok(lf.le(&rf).into()),
                    Op::Eq  => Ok(lf._eq(&rf).into()),
                    Op::Neq => Ok(lf._eq(&rf).not().into()),
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
                                return Err("Verification Error: Potential division by zero.".into());
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
                    _ => Err(format!("Unsupported int operator {:?}", op)),
                }
            }
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let c = expr_to_z3(ctx, arr, cond, env, solver_opt)?.as_bool().unwrap();
            let t = expr_to_z3(ctx, arr, then_branch, env, solver_opt)?;
            let e = expr_to_z3(ctx, arr, else_branch, env, solver_opt)?;
            Ok(c.ite(&t, &e))
        },
        Expr::Let { var, value, body } => {
            let val = expr_to_z3(ctx, arr, value, env, solver_opt)?;
            let old = env.insert(var.clone(), val);
            let res = expr_to_z3(ctx, arr, body, env, solver_opt)?;
            if let Some(v) = old { env.insert(var.clone(), v); } else { env.remove(var); }
            Ok(res)
        },
        Expr::Assign { var, value } => {
            let val = expr_to_z3(ctx, arr, value, env, solver_opt)?;
            env.insert(var.clone(), val.clone());
            Ok(val)
        },
        Expr::Block(stmts) => {
            let mut last = Int::from_i64(ctx, 0).into();
            for stmt in stmts { last = expr_to_z3(ctx, arr, stmt, env, solver_opt)?; }
            Ok(last)
        },
        Expr::While { cond, invariant, body } => {
            // Loop Invariant 検証ロジック (既存)
            if let Some(solver) = solver_opt {
                let inv = expr_to_z3(ctx, arr, invariant, env, None)?.as_bool().unwrap();
                // Base case
                solver.push();
                solver.assert(&inv.not());
                if solver.check() == SatResult::Sat { return Err("Invariant fails initially".into()); }
                solver.pop(1);
                // Inductive step (簡略化)
                let c = expr_to_z3(ctx, arr, cond, env, None)?.as_bool().unwrap();
                solver.push();
                solver.assert(&inv);
                solver.assert(&c);
                expr_to_z3(ctx, arr, body, env, Some(solver))?;
                let inv_after = expr_to_z3(ctx, arr, invariant, env, None)?.as_bool().unwrap();
                solver.assert(&inv_after.not());
                if solver.check() == SatResult::Sat { return Err("Invariant not preserved".into()); }
                solver.pop(1);
            }
            let inv = expr_to_z3(ctx, arr, invariant, env, None)?.as_bool().unwrap();
            let c_not = expr_to_z3(ctx, arr, cond, env, None)?.as_bool().unwrap().not();
            Ok(Bool::and(ctx, &[&inv, &c_not]).into())
        }
        _ => Err("Unsupported expr in Z3 conversion".into()),
    }
}

fn save_visualizer_report(output_dir: &Path, status: &str, name: &str, a: &str, b: &str, reason: &str) {
    let report = json!({ "status": status, "atom": name, "input_a": a, "input_b": b, "reason": reason });
    let _ = fs::create_dir_all(output_dir);
    let _ = fs::write(output_dir.join("report.json"), report.to_string());
}
