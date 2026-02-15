mod parser;
mod verification;
mod codegen;
mod transpiler;

use clap::Parser;
use std::fs;
use std::path::Path;
use crate::transpiler::{TargetLanguage, transpile};

#[derive(Parser)]
#[command(name = "Mumei Compiler", version = "0.1.0")]
struct Cli {
    /// Input .mm file (e.g., example.mm)
    input: String,
    /// Output object name (base name for .ll, .rs, .go, .ts)
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

    println!("🗡️  Mumei: Forging the blade...");

    // --- 1. Parsing (構文解析) ---
    // AST (Abstract Syntax Tree) を生成
    let atom = parser::parse(&source);
    println!("  ✨ [1/4] Polishing Syntax: Atom '{}' identified.", atom.name);

    let output_path = Path::new(&cli.output);
    let output_dir = output_path.parent().unwrap_or(Path::new("."));
    // ファイル名部分（拡張子なし）を取得
    let file_stem = output_path.file_stem().and_then(|s| s.to_str()).unwrap_or(&cli.output);

    // --- 2. Verification (形式検証: Z3) ---
    // ここがガードレール。論理的に正しくないコードはここで遮断されます。
    match verification::verify(&atom, output_dir) {
        Ok(_) => println!("  ⚖️  [2/4] Verification: Passed. The logic is flawless."),
        Err(e) => {
            eprintln!("  ❌ [2/4] Verification: Failed! Flaw detected in logic: {}", e);
            // 検証に失敗した場合は、不完全（危険）な成果物を出さないよう即座に終了
            std::process::exit(1);
        }
    }

    // --- 3. Codegen (低レイヤ生成: LLVM IR) ---
    // 形式検証をパスした「正しい論理」のみがマシンコードへ変換される
    match codegen::compile(&atom, output_path) {
        Ok(_) => println!("  ⚙️  [3/4] Tempering: Done. Created '{}.ll'", file_stem),
        Err(e) => {
            eprintln!("  ❌ [3/4] Tempering: Failed! {}", e);
            std::process::exit(1);
        }
    }

    // --- 4. Transpile (多言語エクスポート) ---
    // 高レイヤ言語への出力
    println!("  🌍 [4/4] Sharpening: Exporting verified Rust, Go, and TypeScript sources...");

    let targets = [
        (TargetLanguage::Rust, "rs"),
        (TargetLanguage::Go, "go"),
        (TargetLanguage::TypeScript, "ts"),
    ];

    for (lang, ext) in targets.iter() {
        let code = transpile(&atom, *lang);
        let out_filename = format!("{}.{}", file_stem, ext);
        let out_full_path = output_dir.join(&out_filename);

        if let Err(e) = fs::write(&out_full_path, code) {
            eprintln!("  ❌ Failed to write {}: {}", out_filename, e);
            std::process::exit(1);
        }
    }

    println!("  ✅ Done. Created '{0}.rs', '{0}.go', '{0}.ts'", file_stem);
    println!("🎉 Blade forged and sharpened successfully.");
}