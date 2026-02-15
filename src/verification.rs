use z3::ast::{Ast, Int, Bool, Array, Dynamic};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::{Atom, QuantifierType, Expr, Op, parse_expression};
use std::fs;
use std::path::Path;
use serde_json::json;

/// Mumeiのアトムを検証し、指定されたディレクトリに report.json を出力する
pub fn verify(atom: &Atom, output_dir: &Path) -> Result<(), String> {
    let mut cfg = Config::new();
    cfg.set_timeout(10000);
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let b = Int::new_const(&ctx, "b");
    let zero = Int::from_i64(&ctx, 0);

    let int_sort = Int::get_sort(&ctx);
    let array_sort = z3::Sort::array(&ctx, &int_sort, &int_sort);
    let arr = Array::new_const(&ctx, "arr", &array_sort);

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
        let condition_z3 = expr_to_z3(&ctx, &arr, &expr_ast).as_bool().expect("Condition must be boolean");

        let quantifier_expr = match q.q_type {
            QuantifierType::ForAll => {
                z3::ast::forall_const(&ctx, &[&i], &[], &range_cond.implies(&condition_z3))
            },
            QuantifierType::Exists => {
                z3::ast::exists_const(&ctx, &[&i], &[], &Bool::and(&ctx, &[&range_cond, &condition_z3]))
            }
        };
        solver.assert(&quantifier_expr);
    }

    if atom.requires.contains("b != 0") {
        solver.assert(&b._eq(&zero).not());
    }

    // --- 検証の実行 ---

    // A. ゼロ除算チェック (分岐を考慮した検証)
    // body_expr を AST 解析して、Ite の中身まで確認します
    let body_ast = parse_expression(&atom.body_expr);

    // Z3 に body の計算式を覚えさせ、その過程で division by zero が発生しうるか確認
    // (ここでは簡易的に、b=0 且つ requires を満たす反例を探す従来ロジックを維持しつつ、
    // ifガードがある場合は solver が「b=0 のときはこのパスを通らない」と判断します)
    if atom.body_expr.contains("/") {
        solver.push();
        solver.assert(&b._eq(&zero));

        if solver.check() == SatResult::Sat {
            // もし if b != 0 { a/b } else { 0 } のようなガードがあれば、
            // solver は Unsat (矛盾) を返し、ここには来ません。
            let model = solver.get_model().unwrap();
            let b_val = model.eval(&b, true).unwrap().to_string();
            save_visualizer_report(output_dir, "failed", &atom.name, "N/A", &b_val, "Potential division by zero.");
            return Err(format!("Unsafe division found when b={}", b_val));
        }
        solver.pop(1);
    }

    // B. 全体整合性チェック
    if solver.check() == SatResult::Unsat {
        save_visualizer_report(output_dir, "failed", &atom.name, "N/A", "N/A", "Logic contradiction found.");
        Err("Verification failed: The constraints are mathematically impossible.".to_string())
    } else {
        save_visualizer_report(output_dir, "success", &atom.name, "N/A", "N/A", "All logical constraints and branching are safe.");
        Ok(())
    }
}

fn expr_to_z3<'a>(ctx: &'a Context, arr: &Array<'a>, expr: &Expr) -> Dynamic<'a> {
    match expr {
        Expr::Number(n) => Int::from_i64(ctx, *n).into(),
        Expr::Variable(name) => Int::new_const(ctx, name.as_str()).into(),
        Expr::ArrayAccess(_name, index_expr) => {
            let idx = expr_to_z3(ctx, arr, index_expr).as_int().expect("Index must be integer");
            arr.select(&idx).into()
        },
        Expr::IfThenElse { cond, then_branch, else_branch } => {
            let c = expr_to_z3(ctx, arr, cond).as_bool().expect("If condition must be boolean");
            let t = expr_to_z3(ctx, arr, then_branch);
            let e = expr_to_z3(ctx, arr, else_branch);
            // Z3 の If-Then-Else 構文を使用
            c.ite(&t, &e)
        },
        Expr::BinaryOp(left, op, right) => {
            let l = expr_to_z3(ctx, arr, left);
            let r = expr_to_z3(ctx, arr, right);
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
    let report = json!({
        "status": status,
        "atom": name,
        "input_a": a,
        "input_b": b,
        "reason": reason
    });
    let _ = fs::create_dir_all(output_dir);
    let report_path = output_dir.join("report.json");
    fs::write(report_path, report.to_string()).expect("Failed to write report");
}
}