use z3::ast::{Ast, Int, Bool, Array, Dynamic};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::{Atom, QuantifierType, Expr, Op, parse_expression};
use std::fs;
use std::path::Path;
use serde_json::json;

/// Mumeiのアトムを検証し、指定されたディレクトリに report.json を出力する
pub fn verify(atom: &Atom, output_dir: &Path) -> Result<(), String> {
    // 1. Z3コンテキストとソルバーの初期化
    let mut cfg = Config::new();
    cfg.set_timeout(10000); // タイムアウト 10秒
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    // --- 基本変数の定義 ---
    let b = Int::new_const(&ctx, "b");
    let zero = Int::from_i64(&ctx, 0);

    // --- 配列（Array）の定義 ---
    let int_sort = Int::get_sort(&ctx);
    let array_sort = z3::Sort::array(&ctx, &int_sort, &int_sort);
    let arr = Array::new_const(&ctx, "arr", &array_sort);

    // 2. 高度な数学的制約 (forall / exists) の構築
    for q in &atom.forall_constraints {
        let i = Int::new_const(&ctx, q.var.as_str());
        let start = Int::from_i64(&ctx, q.start.parse::<i64>().unwrap_or(0));

        let end = if let Ok(val) = q.end.parse::<i64>() {
            Int::from_i64(&ctx, val)
        } else {
            Int::new_const(&ctx, q.end.as_str()) // 変数（len等）
        };

        // 範囲条件: start <= i < end
        let range_cond = Bool::and(&ctx, &[&i.ge(&start), &i.lt(&end)]);

        // --- AST解析器による論理条件の構築 ---
        let expr_ast = parse_expression(&q.condition);
        let condition_z3 = expr_to_z3(&ctx, &arr, &expr_ast).as_bool().expect("Condition must be boolean");

        let quantifier_expr = match q.q_type {
            QuantifierType::ForAll => {
                // ∀i. (range_cond => condition_z3)
                z3::ast::forall_const(&ctx, &[&i], &[], &range_cond.implies(&condition_z3))
            },
            QuantifierType::Exists => {
                // ∃i. (range_cond ∧ condition_z3)
                z3::ast::exists_const(&ctx, &[&i], &[], &Bool::and(&ctx, &[&range_cond, &condition_z3]))
            }
        };
        solver.assert(&quantifier_expr);
    }

    // 3. 基本的な requires 制約 (b != 0 等) の反映
    if atom.requires.contains("b != 0") {
        solver.assert(&b._eq(&zero).not());
    }

    // 4. 検証の実行

    // A. ゼロ除算チェック (個別検証)
    if atom.body_expr.contains("/") {
        solver.push();
        solver.assert(&b._eq(&zero));

        if solver.check() == SatResult::Sat {
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
        save_visualizer_report(output_dir, "success", &atom.name, "N/A", "N/A", "All logical constraints (ForAll, Exists, Implies) are satisfied.");
        Ok(())
    }
}

/// AST (Expr) を再帰的に Z3 の式に変換する
fn expr_to_z3<'a>(ctx: &'a Context, arr: &Array<'a>, expr: &Expr) -> Dynamic<'a> {
    match expr {
        Expr::Number(n) => Int::from_i64(ctx, *n).into(),
        Expr::Variable(name) => {
            // target_id などの特別な変数は実行時に解決
            Int::new_const(ctx, name.as_str()).into()
        },
        Expr::ArrayAccess(_name, index_expr) => {
            let idx = expr_to_z3(ctx, arr, index_expr).as_int().expect("Index must be integer");
            arr.select(&idx).into()
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

/// ビジュアライザー用JSON保存関数
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