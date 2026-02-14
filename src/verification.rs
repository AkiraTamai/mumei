use z3::ast::{Ast, Int, Bool};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::Atom;
use std::fs;
use std::path::Path; // 追加
use serde_json::json;

/// Mumeiのアトムを検証し、指定されたディレクトリに report.json を出力する
pub fn verify(atom: &Atom, output_dir: &Path) -> Result<(), String> {
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let a = Int::new_const(&ctx, "a");
    let b = Int::new_const(&ctx, "b");
    let zero = Int::from_i64(&ctx, 0);

    // 簡易的な制約チェック
    let requires = if atom.requires.contains("b != 0") {
        b._eq(&zero).not()
    } else {
        Bool::from_bool(&ctx, true)
    };

    if atom.body_expr.contains("/") {
        solver.assert(&requires);
        solver.assert(&b._eq(&zero)); // b=0 になる反例を探す

        if solver.check() == SatResult::Sat {
            let model = solver.get_model().unwrap();
            let a_val = model.eval(&a, true).unwrap().to_string();
            let b_val = model.eval(&b, true).unwrap().to_string();

            // 指定された出力先に失敗レポートを保存
            save_visualizer_report(output_dir, "failed", &atom.name, &a_val, &b_val, "Potential division by zero.");

            return Err(format!("Unsafe division found when b={}", b_val));
        }
    }

    // 指定された出力先に成功レポートを保存
    save_visualizer_report(output_dir, "success", &atom.name, "N/A", "N/A", "The logic is pure.");

    Ok(())
}

/// ビジュアライザー用JSON保存関数（出力先ディレクトリを指定可能に）
fn save_visualizer_report(output_dir: &Path, status: &str, name: &str, a: &str, b: &str, reason: &str) {
    let report = json!({
        "status": status,
        "atom": name,
        "input_a": a,
        "input_b": b,
        "reason": reason
    });

    // 出力先ディレクトリが存在することを確認（一時ディレクトリの場合は既に存在しているはず）
    let _ = fs::create_dir_all(output_dir);

    // report.json を指定されたディレクトリ直下に作成
    let report_path = output_dir.join("report.json");
    fs::write(report_path, report.to_string()).expect("Failed to write report");
}