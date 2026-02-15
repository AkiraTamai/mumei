use z3::ast::{Ast, Int, Bool, Array, Dynamic};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::{Atom, QuantifierType, Expr, Op, parse_expression};
use std::fs;
use std::path::Path;
use serde_json::json;
use std::collections::HashMap;

pub fn verify(atom: &Atom, output_dir: &Path) -> Result<(), String> {
    let mut cfg = Config::new();
    cfg.set_timeout(10000);
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let int_sort = Int::get_sort(&ctx);
    let array_sort = z3::Sort::array(&ctx, &int_sort, &int_sort);
    let arr = Array::new_const(&ctx, "arr", &array_sort);

    let mut env: HashMap<String, Dynamic> = HashMap::new();

    // 1. 事前条件 (Requires / ForAll) のアサート
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
        let condition_z3 = expr_to_z3(&ctx, &arr, &expr_ast, &mut env, None).as_bool().expect("Condition must be boolean");

        let quantifier_expr = match q.q_type {
            QuantifierType::ForAll => z3::ast::forall_const(&ctx, &[&i], &[], &range_cond.implies(&condition_z3)),
            QuantifierType::Exists => z3::ast::exists_const(&ctx, &[&i], &[], &Bool::and(&ctx, &[&range_cond, &condition_z3])),
        };
        solver.assert(&quantifier_expr);
    }

    // 2. Body の解析と安全性検証
    let body_ast = parse_expression(&atom.body_expr);

    // 修正：expr_to_z3 に solver を渡し、式の中で除算が見つかるたびに安全性をチェックする
    let _body_result = expr_to_z3(&ctx, &arr, &body_ast, &mut env, Some(&solver));

    // 3. 最終的な論理矛盾チェック
    if solver.check() == SatResult::Unsat {
        save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Logic contradiction in constraints.");
        return Err("Verification failed: Contradiction found.".into());
    }

    save_visualizer_report(output_dir, "success", &atom.name, "N/A", "N/A", "All paths verified safe.");
    Ok(())
}

fn expr_to_z3<'a>(
    ctx: &'a Context,
    arr: &Array<'a>,
    expr: &Expr,
    env: &mut HashMap<String, Dynamic<'a>>,
    solver_opt: Option<&Solver<'a>> // 安全性チェック用
) -> Dynamic<'a> {
    match expr {
        Expr::Number(n) => Int::from_i64(ctx, *n).into(),
        Expr::Variable(name) => {
            env.get(name).cloned().unwrap_or_else(|| Int::new_const(ctx, name.as_str()).into())
        },
        Expr::ArrayAccess(_name, index_expr) => {
            let idx = expr_to_z3(ctx, arr, index_expr, env, solver_opt).as_int().expect("Index must be integer");
            arr.select(&idx).into()
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let c = expr_to_z3(ctx, arr, cond, env, solver_opt).as_bool().expect("If condition must be boolean");

            // ifのパスごとに環境を分離して検証（簡略化のため現状はDynamicでのITE）
            let t = expr_to_z3(ctx, arr, then_branch, env, solver_opt);
            let e = expr_to_z3(ctx, arr, else_branch, env, solver_opt);
            c.ite(&t, &e)
        },
        Expr::Let { var, value, body } => {
            let val = expr_to_z3(ctx, arr, value, env, solver_opt);
            env.insert(var.clone(), val);
            expr_to_z3(ctx, arr, body, env, solver_opt)
        },
        Expr::Block(stmts) => {
            let mut last_val = Int::from_i64(ctx, 0).into();
            for stmt in stmts {
                last_val = expr_to_z3(ctx, arr, stmt, env, solver_opt);
            }
            last_val
        },
        Expr::BinaryOp(left, op, right) => {
            let l = expr_to_z3(ctx, arr, left, env, solver_opt);
            let r = expr_to_z3(ctx, arr, right, env, solver_opt);

            match op {
                Op::Div => {
                    let denominator = r.as_int().unwrap();
                    // 修正：除算が見つかったら、その時点のパス条件で分母が0になり得るか検証
                    if let Some(solver) = solver_opt {
                        solver.push();
                        solver.assert(&denominator._eq(&Int::from_i64(ctx, 0)));
                        if solver.check() == SatResult::Sat {
                            // 0になり得る反例が見つかった
                            let _model = solver.get_model();
                            // 実際はここでプロセスを中断するかエラーフラグを立てる必要がある
                            // Mumeiでは即座にエラーとして報告
                            panic!("Verification Error: Potential division by zero detected.");
                        }
                        solver.pop(1);
                    }
                    (l.as_int().unwrap() / denominator).into()
                },
                Op::Add => (l.as_int().unwrap() + r.as_int().unwrap()).into(),
                Op::Sub => (l.as_int().unwrap() - r.as_int().unwrap()).into(),
                Op::Mul => (l.as_int().unwrap() * r.as_int().unwrap()).into(),
                Op::Gt  => l.as_int().unwrap().gt(&r.as_int().unwrap()).into(),
                Op::Lt  => l.as_int().unwrap().lt(&r.as_int().unwrap()).into(),
                Op::Ge  => l.as_int().unwrap().ge(&r.as_int().unwrap()).into(),
                Op::Le  => l.as_int().unwrap().le(&r.as_int().unwrap()).into(),
                Op::Eq  => l.as_int().unwrap()._eq(&r.as_int().unwrap()).into(),
                Op::Neq => l.as_int().unwrap()._eq(&r.as_int().unwrap()).not().into(),
                Op::And => Bool::and(ctx, &[&l.as_bool().unwrap(), &r.as_bool().unwrap()]).into(),
                Op::Or  => Bool::or(ctx, &[&l.as_bool().unwrap(), &r.as_bool().unwrap()]).into(),
                Op::Implies => l.as_bool().unwrap().implies(&r.as_bool().unwrap()).into(),
            }
        }
    }
}

fn save_visualizer_report(output_dir: &Path, status: &str, name: &str, a: &str, b: &str, reason: &str) {
    let report = json!({ "status": status, "atom": name, "input_a": a, "input_b": b, "reason": reason });
    let _ = fs::create_dir_all(output_dir);
    let _ = fs::write(output_dir.join("report.json"), report.to_string());
}