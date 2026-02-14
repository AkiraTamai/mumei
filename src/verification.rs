use z3::ast::{Ast, Int, Bool};
use z3::{Config, Context, Solver, SatResult};
use crate::parser::Atom;
use std::fs; // 追加
use serde_json::json; // 追加 (Cargo.tomlに serde_json = "1.0" が必要)

pub fn verify(atom: &Atom) -> Result<(), String> {
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let a = Int::new_const(&ctx, "a"); // 可視化のために 'a' も定義
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
        solver.assert(&b._eq(&zero)); // b=0 になるケースを探す

        if solver.check() == SatResult::Sat {
            let model = solver.get_model().unwrap();
            let a_val = model.eval(&a, true).unwrap().to_string();
            let b_val = model.eval(&b, true).unwrap().to_string();

            // ★差し込みポイント1：失敗の報告を書き出し
            save_visualizer_report("failed", &atom.name, &a_val, &b_val, "Potential division by zero.");

            return Err(format!("Unsafe division found when b={}", b_val));
        }
    }

    // ★差し込みポイント2：成功の報告を書き出し
    save_visualizer_report("success", &atom.name, "N/A", "N/A", "The logic is pure.");

    Ok(())
}

// ビジュアライザー用JSON保存関数
fn save_visualizer_report(status: &str, name: &str, a: &str, b: &str, reason: &str) {
    let report = json!({
        "status": status,
        "atom": name,
        "input_a": a,
        "input_b": b,
        "reason": reason
    });
    // visualizerディレクトリがない場合のエラーを防ぐため、作成を試みる
    let _ = fs::create_dir_all("visualizer");
    fs::write("visualizer/report.json", report.to_string()).expect("Failed to write report");
}
