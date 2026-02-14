mod parser;
mod verification;
mod codegen;

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
    println!("  ✨ [1/3] Polishing Syntax: Atom '{}' identified.", atom.name);

    // 2. Verification (The Ritual of Truth)
    match verification::verify(&atom) {
        Ok(_) => println!("  ⚖️  [2/3] Verification: Passed. The logic is flawless."),
        Err(e) => {
            eprintln!("  ❌ [2/3] Verification: Failed! Flaw detected in logic: {}", e);
            std::process::exit(1);
        }
    }

    // 3. Codegen (The Tempering)
    match codegen::compile(&atom, Path::new(&cli.output)) {
        Ok(_) => println!("  ⚙️  [3/3] Tempering: Done. Created '{}.ll'", cli.output),
        Err(e) => eprintln!("  ❌ [3/3] Tempering: Failed! {}", e),
    }

    println!("🎉 Blade forged successfully.");
}