mod parser;
mod verification;
mod codegen;
mod transpiler;

use clap::Parser;
use std::fs;
use std::path::Path;
use crate::transpiler::{TargetLanguage, transpile};
use crate::parser::Item;

#[derive(Parser)]
#[command(name = "Mumei Compiler", version = "0.1.0")]
struct Cli {
    /// Input .mm file (e.g., example.mm)
    input: String,
    /// Output base name (for .ll, .rs, .go, .ts)
    #[arg(short, long, default_value = "katana")]
    output: String,
}

fn main() {
    let cli = Cli::parse();

    // ソースファイルの読み込み
    let source = fs::read_to_string(&cli.input).unwrap_or_else(|_| {
        eprintln!("❌ Error: Could not read Mumei source file '{}'", cli.input);
        std::process::exit(1);
    });

    println!("🗡️  Mumei: Forging the blade (Type System 2.0 enabled)...");

    // --- 1. Parsing (構文解析) ---
    let items = parser::parse_module(&source);

    let output_path = Path::new(&cli.output);
    let output_dir = output_path.parent().unwrap_or(Path::new("."));
    // ベースとなるファイル名（例: katana）
    let file_stem = output_path.file_stem().and_then(|s| s.to_str()).unwrap_or(&cli.output);

    let mut atom_count = 0;

    // 全ての Atom のコードを結合して出力するためのバッファ (Transpiler用)
    let mut rust_bundle = String::new();
    let mut go_bundle = String::new();
    let mut ts_bundle = String::new();

    for item in items {
        match item {
            // --- 精緻型の登録 ---
            Item::TypeDef(refined_type) => {
                println!("  ✨ Registered Refined Type: '{}' ({})", refined_type.name, refined_type._base_type);
                if let Err(e) = verification::register_type(&refined_type) {
                    eprintln!("  ❌ Type Registration Failed: {}", e);
                    std::process::exit(1);
                }
            }

            // --- 構造体定義の登録 ---
            Item::StructDef(struct_def) => {
                let field_names: Vec<&str> = struct_def.fields.iter().map(|f| f.name.as_str()).collect();
                println!("  🏗️  Registered Struct: '{}' (fields: {})", struct_def.name, field_names.join(", "));
                if let Err(e) = verification::register_struct(&struct_def) {
                    eprintln!("  ❌ Struct Registration Failed: {}", e);
                    std::process::exit(1);
                }
            }

            // --- Atom の処理 ---
            Item::Atom(atom) => {
                atom_count += 1;
                println!("  ✨ [1/4] Polishing Syntax: Atom '{}' identified.", atom.name);

                // --- 2. Verification (形式検証: Z3 + StdLib) ---
                // 配列境界チェックや浮動小数点演算の検証を含む
                match verification::verify(&atom, output_dir) {
                    Ok(_) => println!("  ⚖️  [2/4] Verification: Passed. Logic verified with Z3."),
                    Err(e) => {
                        eprintln!("  ❌ [2/4] Verification: Failed! Flaw detected: {}", e);
                        std::process::exit(1);
                    }
                }

                // --- 3. Codegen (LLVM 18 + Floating Point) ---
                // 各 Atom ごとに .ll ファイルを生成（またはモジュールを統合する拡張も可能）
                let atom_output_path = output_dir.join(format!("{}_{}", file_stem, atom.name));
                match codegen::compile(&atom, &atom_output_path) {
                    Ok(_) => println!("  ⚙️  [3/4] Tempering: Done. Compiled '{}' to LLVM IR.", atom.name),
                    Err(e) => {
                        eprintln!("  ❌ [3/4] Tempering: Failed! Codegen error: {}", e);
                        std::process::exit(1);
                    }
                }

                // --- 4. Transpile (多言語エクスポート) ---
                // バンドル用に各言語のコードを生成
                rust_bundle.push_str(&transpile(&atom, TargetLanguage::Rust));
                rust_bundle.push_str("\n\n");

                go_bundle.push_str(&transpile(&atom, TargetLanguage::Go));
                go_bundle.push_str("\n\n");

                ts_bundle.push_str(&transpile(&atom, TargetLanguage::TypeScript));
                ts_bundle.push_str("\n\n");
            }
        }
    }

    // 各言語のファイルを一括書き出し
    if atom_count > 0 {
        println!("  🌍 [4/4] Sharpening: Exporting verified sources...");

        let files = [
            (rust_bundle, "rs"),
            (go_bundle, "go"),
            (ts_bundle, "ts"),
        ];

        for (code, ext) in files {
            let out_filename = format!("{}.{}", file_stem, ext);
            let out_full_path = output_dir.join(&out_filename);
            if let Err(e) = fs::write(&out_full_path, code) {
                eprintln!("  ❌ Failed to write {}: {}", out_filename, e);
                std::process::exit(1);
            }
        }
        println!("  ✅ Done. Created '{0}.rs', '{0}.go', '{0}.ts'", file_stem);
        println!("🎉 Blade forged successfully with {} atoms.", atom_count);
    } else {
        println!("⚠️  Warning: No atoms found in the source file.");
    }
}
