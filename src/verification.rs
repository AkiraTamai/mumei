use z3::ast::{Ast, Int, Bool, Array, Dynamic};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::{Atom, QuantifierType, Expr, Op, parse_expression, RefinedType};
use std::fs;
use std::path::Path;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// --- 型環境のグローバル管理 ---
// main.rs で登録された精緻型を保持するためのスレッドセーフなグローバルマップ
static TYPE_ENV: Lazy<Mutex<HashMap<String, RefinedType>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// 精緻型をグローバルな型環境に登録する
pub fn register_type(refined_type: &RefinedType) -> Result<(), String> {
    let mut env = TYPE_ENV.lock().map_err(|_| "Failed to lock TYPE_ENV")?;
    env.insert(refined_type.name.clone(), refined_type.clone());
    Ok(())
}

pub fn verify(atom: &Atom, output_dir: &Path) -> Result<(), String> {
    let mut cfg = Config::new();
    cfg.set_timeout_msec(10000);
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let int_sort = z3::Sort::int(&ctx);
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
    // ここでは簡易的に、変数名から型を推論するか、命名規則等で精緻型を適用する仕組みを想定
    // 将来的には Atom 構造体に引数の型情報を持たせることが望ましい
    let type_defs = TYPE_ENV.lock().map_err(|_| "Failed to lock TYPE_ENV")?;
    for param in &atom.params {
        // 例: パラメータ名に対応する型が登録されていれば制約を適用
        // 実際の実装では `arg: Type` のパース結果を利用するように拡張が必要
        if let Some(refined) = type_defs.get(param) {
            apply_refinement_constraint(&ctx, &arr, &solver, param, refined)?;
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
                save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Postcondition (ensures) violated.");
                return Err("Verification Error: Postcondition (ensures) is not satisfied.".into());
            }
            solver.pop(1);
        }
        env.remove("result");
    }

    if solver.check() == SatResult::Unsat {
        save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Logic contradiction in constraints.");
        return Err("Verification failed: Contradiction found.".into());
    }

    save_visualizer_report(output_dir, "success", &atom.name, "N/A", "N/A", "All paths verified safe.");
    Ok(())
}

/// 特定の変数に対して精緻型の制約を Z3 ソルバーに適用する
fn apply_refinement_constraint<'a>(
    ctx: &'a Context,
    arr: &Array<'a>,
    solver: &Solver<'a>,
    var_name: &str,
    refined: &RefinedType
) -> Result<(), String> {
    let mut local_env = HashMap::new();
    let var_z3 = Int::new_const(ctx, var_name);

    // 制約式内の `v` (operand) を現在の変数名に置き換えるための環境設定
    local_env.insert(refined.operand.clone(), var_z3.into());

    let predicate_ast = parse_expression(&refined.predicate_raw);
    let predicate_z3 = expr_to_z3(ctx, arr, &predicate_ast, &mut local_env, None)?
        .as_bool().ok_or(format!("Predicate for type {} must be boolean", refined.name))?;

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
        Expr::Variable(name) => {
            let val = env.get(name).cloned().unwrap_or_else(|| Int::new_const(ctx, name.as_str()).into());

            // 変数参照時、もしその変数名が精緻型として登録されている名前を含んでいれば制約を適用するロジック（暫定）
            // 将来的にはシンボルテーブルで型情報を厳密に管理する
            Ok(val)
        },
        Expr::ArrayAccess(_name, index_expr) => {
            let idx = expr_to_z3(ctx, arr, index_expr, env, solver_opt)?
                .as_int().ok_or("Index must be integer")?;
            Ok(arr.select(&idx).into())
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let c = expr_to_z3(ctx, arr, cond, env, solver_opt)?
                .as_bool().ok_or("If condition must be boolean")?;

            let t = if let Some(solver) = solver_opt {
                solver.push();
                solver.assert(&c);
                let res = expr_to_z3(ctx, arr, then_branch, env, solver_opt)?;
                solver.pop(1);
                res
            } else {
                expr_to_z3(ctx, arr, then_branch, env, solver_opt)?
            };

            let e = if let Some(solver) = solver_opt {
                solver.push();
                solver.assert(&c.not());
                let res = expr_to_z3(ctx, arr, else_branch, env, solver_opt)?;
                solver.pop(1);
                res
            } else {
                expr_to_z3(ctx, arr, else_branch, env, solver_opt)?
            };

            Ok(c.ite(&t, &e))
        },
        Expr::While { cond, invariant, body } => {
            if let Some(solver) = solver_opt {
                let inv_z3 = expr_to_z3(ctx, arr, invariant, env, None)?
                    .as_bool().ok_or("Invariant must be boolean")?;
                solver.push();
                solver.assert(&inv_z3.not());
                if solver.check() == SatResult::Sat {
                    solver.pop(1);
                    return Err("Verification Error: Loop invariant does not hold initially.".into());
                }
                solver.pop(1);

                solver.push();
                let cond_z3 = expr_to_z3(ctx, arr, cond, env, None)?
                    .as_bool().ok_or("Loop condition must be boolean")?;
                solver.assert(&inv_z3);
                solver.assert(&cond_z3);

                let env_before_body = env.clone();
                match body.as_ref() {
                    Expr::Block(stmts) => {
                        for stmt in stmts {
                            match stmt {
                                Expr::Let { var, value, .. } => {
                                    let val = expr_to_z3(ctx, arr, value, env, Some(solver))?;
                                    env.insert(var.clone(), val);
                                },
                                _ => {
                                    expr_to_z3(ctx, arr, stmt, env, Some(solver))?;
                                }
                            }
                        }
                    },
                    _ => {
                        expr_to_z3(ctx, arr, body, env, Some(solver))?;
                    }
                }

                let inv_after = expr_to_z3(ctx, arr, invariant, env, None)?
                    .as_bool().ok_or("Invariant must be boolean")?;

                solver.assert(&inv_after.not());
                if solver.check() == SatResult::Sat {
                    solver.pop(1);
                    *env = env_before_body;
                    return Err("Verification Error: Loop invariant is not preserved by the body.".into());
                }
                solver.pop(1);
                *env = env_before_body;
            }

            let final_inv = expr_to_z3(ctx, arr, invariant, env, None)?
                .as_bool().ok_or("Invariant must be boolean")?;
            let final_cond_not = expr_to_z3(ctx, arr, cond, env, None)?
                .as_bool().ok_or("Loop condition must be boolean")?
                .not();
            Ok(Bool::and(ctx, &[&final_inv, &final_cond_not]).into())
        },
        Expr::Let { var, value, body } => {
            let val = expr_to_z3(ctx, arr, value, env, solver_opt)?;
            let old_val = env.insert(var.clone(), val);
            let res = expr_to_z3(ctx, arr, body, env, solver_opt)?;
            if let Some(prev) = old_val { env.insert(var.clone(), prev); }
            else { env.remove(var); }
            Ok(res)
        },
        Expr::Assign { var, value } => {
            let val = expr_to_z3(ctx, arr, value, env, solver_opt)?;
            env.insert(var.clone(), val.clone());
            Ok(val)
        },
        Expr::Block(stmts) => {
            let env_snapshot = env.clone();
            let mut last_val: Dynamic<'a> = Int::from_i64(ctx, 0).into();
            for stmt in stmts {
                match stmt {
                    Expr::Let { var, value, .. } => {
                        let val = expr_to_z3(ctx, arr, value, env, solver_opt)?;
                        env.insert(var.clone(), val.clone());
                        last_val = val;
                    },
                    _ => {
                        last_val = expr_to_z3(ctx, arr, stmt, env, solver_opt)?;
                    }
                }
            }
            *env = env_snapshot;
            Ok(last_val)
        },
        Expr::BinaryOp(left, op, right) => {
            let l = expr_to_z3(ctx, arr, left, env, solver_opt)?;
            let r = expr_to_z3(ctx, arr, right, env, solver_opt)?;

            match op {
                Op::Div => {
                    let denominator = r.as_int().ok_or("Division operand must be integer")?;
                    if let Some(solver) = solver_opt {
                        solver.push();
                        solver.assert(&denominator._eq(&Int::from_i64(ctx, 0)));
                        if solver.check() == SatResult::Sat {
                            solver.pop(1);
                            return Err("Verification Error: Potential division by zero detected.".into());
                        }
                        solver.pop(1);
                    }
                    Ok((l.as_int().ok_or("Division operand must be integer")? / denominator).into())
                },
                Op::Add => Ok((l.as_int().ok_or("Operand must be integer")? + r.as_int().ok_or("Operand must be integer")?).into()),
                Op::Sub => Ok((l.as_int().ok_or("Operand must be integer")? - r.as_int().ok_or("Operand must be integer")?).into()),
                Op::Mul => Ok((l.as_int().ok_or("Operand must be integer")? * r.as_int().ok_or("Operand must be integer")?).into()),
                Op::Gt  => Ok(l.as_int().ok_or("Operand must be integer")?.gt(&r.as_int().ok_or("Operand must be integer")?).into()),
                Op::Lt  => Ok(l.as_int().ok_or("Operand must be integer")?.lt(&r.as_int().ok_or("Operand must be integer")?).into()),
                Op::Ge  => Ok(l.as_int().ok_or("Operand must be integer")?.ge(&r.as_int().ok_or("Operand must be integer")?).into()),
                Op::Le  => Ok(l.as_int().ok_or("Operand must be integer")?.le(&r.as_int().ok_or("Operand must be integer")?).into()),
                Op::Eq  => Ok(l.as_int().ok_or("Operand must be integer")?._eq(&r.as_int().ok_or("Operand must be integer")?).into()),
                Op::Neq => Ok(l.as_int().ok_or("Operand must be integer")?._eq(&r.as_int().ok_or("Operand must be integer")?).not().into()),
                Op::And => Ok(Bool::and(ctx, &[&l.as_bool().ok_or("Operand must be boolean")?, &r.as_bool().ok_or("Operand must be boolean")?]).into()),
                Op::Or  => Ok(Bool::or(ctx, &[&l.as_bool().ok_or("Operand must be boolean")?, &r.as_bool().ok_or("Operand must be boolean")?]).into()),
                Op::Implies => Ok(l.as_bool().ok_or("Operand must be boolean")?.implies(&r.as_bool().ok_or("Operand must be boolean")?).into()),
            }
        }
    }
}

fn save_visualizer_report(output_dir: &Path, status: &str, name: &str, a: &str, b: &str, reason: &str) {
    let report = json!({ "status": status, "atom": name, "input_a": a, "input_b": b, "reason": reason });
    let _ = fs::create_dir_all(output_dir);
    let _ = fs::write(output_dir.join("report.json"), report.to_string());
}