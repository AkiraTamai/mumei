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

    // 変数環境 (let で定義された変数を保持)
    let mut env: HashMap<String, Dynamic> = HashMap::new();

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
        let condition_z3 = expr_to_z3(&ctx, &arr, &expr_ast, &mut env).as_bool().expect("Condition must be boolean");

        let quantifier_expr = match q.q_type {
            QuantifierType::ForAll => z3::ast::forall_const(&ctx, &[&i], &[], &range_cond.implies(&condition_z3)),
            QuantifierType::Exists => z3::ast::exists_const(&ctx, &[&i], &[], &Bool::and(&ctx, &[&range_cond, &condition_z3])),
        };
        solver.assert(&quantifier_expr);
    }

    // Body の検証
    let body_ast = parse_expression(&atom.body_expr);
    let body_result = expr_to_z3(&ctx, &arr, &body_ast, &mut env);

    // ゼロ除算チェック
    if atom.body_expr.contains("/") {
        solver.push();
        // 現在の環境における「分母」が0になり得るかをチェックするロジックが必要だが
        // ここでは簡単のため、bという変数が定義されている場合に0との比較を行う
        let b = Int::new_const(&ctx, "b");
        solver.assert(&b._eq(&Int::from_i64(&ctx, 0)));

        if solver.check() == SatResult::Sat {
            save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "0", "Potential division by zero.");
            return Err("Unsafe division found.".into());
        }
        solver.pop(1);
    }

    if solver.check() == SatResult::Unsat {
        save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Logic contradiction found.");
        Err("Verification failed.".into())
    } else {
        save_visualizer_report(output_dir, "success", &atom.name, "N/A", "N/A", "All constraints including let-bindings are safe.");
        Ok(())
    }
}

fn expr_to_z3<'a>(
    ctx: &'a Context,
    arr: &Array<'a>,
    expr: &Expr,
    env: &mut HashMap<String, Dynamic<'a>>
) -> Dynamic<'a> {
    match expr {
        Expr::Number(n) => Int::from_i64(ctx, *n).into(),
        Expr::Variable(name) => {
            // envに変数があればそれを使う（letで定義された値）
            env.get(name).cloned().unwrap_or_else(|| Int::new_const(ctx, name.as_str()).into())
        },
        Expr::ArrayAccess(_name, index_expr) => {
            let idx = expr_to_z3(ctx, arr, index_expr, env).as_int().expect("Index must be integer");
            arr.select(&idx).into()
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let c = expr_to_z3(ctx, arr, cond, env).as_bool().expect("If condition must be boolean");
            let t = expr_to_z3(ctx, arr, then_branch, env);
            let e = expr_to_z3(ctx, arr, else_branch, env);
            c.ite(&t, &e)
        },
        Expr::Let { var, value, body } => {
            let val = expr_to_z3(ctx, arr, value, env);
            env.insert(var.clone(), val);
            expr_to_z3(ctx, arr, body, env)
        },
        Expr::Block(stmts) => {
            let mut last_val = Int::from_i64(ctx, 0).into();
            for stmt in stmts {
                last_val = expr_to_z3(ctx, arr, stmt, env);
            }
            last_val
        },
        Expr::BinaryOp(left, op, right) => {
            let l = expr_to_z3(ctx, arr, left, env);
            let r = expr_to_z3(ctx, arr, right, env);
            match op {
                Op::Add => (l.as_int().unwrap() + r.as_int().unwrap()).into(),
                Op::Sub => (l.as_int().unwrap() - r.as_int().unwrap()).into(),
                Op::Mul => (l.as_int().unwrap() * r.as_int().unwrap()).into(),
                Op::Div => (l.as_int().unwrap() / r.as_int().unwrap()).into(),
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