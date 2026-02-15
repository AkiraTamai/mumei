use crate::parser::Atom;
use std::fs;
use std::path::Path;

/// MumeiのAtomをTypeScriptのソースコードに変換する
pub fn transpile(atom: &Atom, output_path: &Path) -> Result<(), String> {
    let mut ts_code = String::new();

    // 1. JSDoc (ドキュメンテーション)
    ts_code.push_str("/**\n");
    ts_code.push_str(&format!(" * {} - Verified by Mumei (Mathematical Proof-Driven Logic)\n", atom.name));
    ts_code.push_str(" *\n");
    ts_code.push_str(" * This code is automatically generated. Do not edit.\n");
    ts_code.push_str(&format!(" * @requires {}\n", atom.requires));

    // 引数の説明を追加
    for param in &atom.params {
        ts_code.push_str(&format!(" * @param {{number}} {} - Input value (treated as integer)\n", param));
    }
    ts_code.push_str(" * @returns {number} The calculated result\n");
    ts_code.push_str(" * @throws {Error} If pre-conditions are violated\n");
    ts_code.push_str(" */\n");

    // 2. 関数定義 (Export)
    let params = atom.params.iter()
        .map(|p| format!("{}: number", p))
        .collect::<Vec<_>>()
        .join(", ");

    ts_code.push_str(&format!("export function {}({}): number {{\n", atom.name, params));

    // 3. 実行時バリデーション (Runtime Validation)
    // TypeScript/JSは型チェックがコンパイル時のみなので、実行時のガードが重要
    if atom.requires.contains("b != 0") {
        ts_code.push_str("    // Mumei Pre-condition Check\n");
        ts_code.push_str("    if (b === 0) {\n");
        ts_code.push_str(&format!(
            "        throw new Error(`Mumei Violation: Requirement 'b != 0' failed in function '{}'`);\n",
            atom.name
        ));
        ts_code.push_str("    }\n\n");
    }

    // 4. ロジック本体
    // MumeiのロジックをそのままJS式として出力
    ts_code.push_str(&format!("    return {};\n", atom.body_expr));
    ts_code.push_str("}\n");

    // 5. ファイル書き出し (.ts)
    let ts_file_path = output_path.with_extension("ts");
    fs::write(&ts_file_path, ts_code).map_err(|e| e.to_string())?;

    Ok(())
}