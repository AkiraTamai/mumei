mod parser;
mod verification;
mod codegen;
mod transpiler; // ★追加: トランスパイラモジュールの宣言

use clap::Parser;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "Mumei Compiler", version = "0.1.0")]
struct Cli {
    /// Input .mm file
    input: String,
    /// Output object name
    #[arg(short, long, default_value = "katana")]
    output: String,
}

fn main() {
    let cli = Cli::parse();
    let source = fs::read_to_string(&cli.input).expect("Failed to read Mumei source file");

    println!("🗡️  Mumei: Forging the blade...");

    // 1. Parsing
    let atom = parser::parse(&source);
    println!("  ✨ [1/4] Polishing Syntax: Atom '{}' identified.", atom.name);

    // 出力先ファイルの親ディレクトリを取得（一時ディレクトリ対応）
    let output_path = Path::new(&cli.output);
    let output_dir = output_path.parent().unwrap_or(Path::new("."));

    // 2. Verification (The Ritual of Truth)
    // 第二引数に output_dir を渡す
    match verification::verify(&atom, output_dir) {
        Ok(_) => println!("  ⚖️  [2/4] Verification: Passed. The logic is flawless."),
        Err(e) => {
            eprintln!("  ❌ [2/4] Verification: Failed! Flaw detected in logic: {}", e);
            std::process::exit(1);
        }
    }

    // 3. Codegen (The Tempering - LLVM IR)
    match codegen::compile(&atom, Path::new(&cli.output)) {
        Ok(_) => println!("  ⚙️  [3/4] Tempering: Done. Created '{}.ll'", cli.output),
        Err(e) => {
            eprintln!("  ❌ [3/4] Tempering: Failed! {}", e);
            std::process::exit(1);
        }
    }

    // 4. Transpile (The Sharpening - Rust Source) ★追加
    println!("  🦀 [4/4] Sharpening: Exporting verified Rust source...");
    match transpiler::transpile_to_rust(&atom, Path::new(&cli.output)) {
        Ok(_) => println!("  ✅ Done. Created '{}.rs'", cli.output),
        Err(e) => {
            eprintln!("  ❌ Transpiling failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("🎉 Blade forged and sharpened successfully.");
}