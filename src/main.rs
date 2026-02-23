mod ast;
mod parser;
mod verification;
mod codegen;
mod transpiler;
mod resolver;

use clap::Parser;
use std::fs;
use std::path::Path;
use crate::transpiler::{TargetLanguage, transpile, transpile_enum, transpile_struct, transpile_module_header};
use crate::parser::{Item, ImportDecl};

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

    println!("🗡️  Mumei: Forging the blade (Type System 2.0 + Generics enabled)...");

    // --- 1. Parsing (構文解析) ---
    let items = parser::parse_module(&source);

    // --- 1.5 Resolve (依存解決) ---
    // import 宣言を処理し、依存モジュールの型・構造体・atom を ModuleEnv に登録
    let mut module_env = verification::ModuleEnv::new();
    let input_path = Path::new(&cli.input);
    let base_dir = input_path.parent().unwrap_or(Path::new("."));
    if let Err(e) = resolver::resolve_imports(&items, base_dir, &mut module_env) {
        eprintln!("  ❌ Import Resolution Failed: {}", e);
        std::process::exit(1);
    }

    // --- 1.7 Monomorphization (単相化) ---
    // ジェネリック定義を収集し、使用箇所の具体型で展開する
    let mut mono = ast::Monomorphizer::new();
    mono.collect(&items);
    let items = if mono.has_generics() {
        let mono_items = mono.monomorphize(&items);
        println!("  🔬 Monomorphization: {} generic instance(s) expanded.", mono.instances().len());
        mono_items
    } else {
        items
    };

    let output_path = Path::new(&cli.output);
    let output_dir = output_path.parent().unwrap_or(Path::new("."));
    // ベースとなるファイル名（例: katana）
    let file_stem = output_path.file_stem().and_then(|s| s.to_str()).unwrap_or(&cli.output);

    let mut atom_count = 0;

    // --- Phase 0: ModuleEnv に全定義を登録 ---
    let mut imports: Vec<ImportDecl> = Vec::new();
    for item in &items {
        match item {
            Item::Import(decl) => imports.push(decl.clone()),
            Item::TypeDef(refined_type) => module_env.register_type(refined_type),
            Item::StructDef(struct_def) => module_env.register_struct(struct_def),
            Item::EnumDef(enum_def) => module_env.register_enum(enum_def),
            Item::Atom(atom) => module_env.register_atom(atom),
            Item::TraitDef(trait_def) => module_env.register_trait(trait_def),
            Item::ImplDef(impl_def) => module_env.register_impl(impl_def),
        }
    }

    // 全ての Atom のコードを結合して出力するためのバッファ (Transpiler用)
    // import 宣言がある場合、各言語のモジュールヘッダーを先頭に挿入
    let mut rust_bundle = transpile_module_header(&imports, file_stem, TargetLanguage::Rust);
    let mut go_bundle = transpile_module_header(&imports, file_stem, TargetLanguage::Go);
    let mut ts_bundle = transpile_module_header(&imports, file_stem, TargetLanguage::TypeScript);

    for item in &items {
        match item {
            // --- import 宣言（resolver で処理済み） ---
            Item::Import(import_decl) => {
                let alias_str = import_decl.alias.as_deref().unwrap_or("(none)");
                println!("  📦 Import: '{}' as '{}'", import_decl.path, alias_str);
            }

            // --- 精緻型の登録 ---
            Item::TypeDef(refined_type) => {
                println!("  ✨ Registered Refined Type: '{}' ({})", refined_type.name, refined_type._base_type);
            }

            // --- 構造体定義の登録 + トランスパイル ---
            Item::StructDef(struct_def) => {
                let field_names: Vec<&str> = struct_def.fields.iter().map(|f| f.name.as_str()).collect();
                println!("  🏗️  Registered Struct: '{}' (fields: {})", struct_def.name, field_names.join(", "));
                // 構造体定義をトランスパイル出力に含める
                rust_bundle.push_str(&transpile_struct(struct_def, TargetLanguage::Rust));
                rust_bundle.push_str("\n\n");
                go_bundle.push_str(&transpile_struct(struct_def, TargetLanguage::Go));
                go_bundle.push_str("\n\n");
                ts_bundle.push_str(&transpile_struct(struct_def, TargetLanguage::TypeScript));
                ts_bundle.push_str("\n\n");
            }

            // --- Enum 定義の登録 + トランスパイル ---
            Item::EnumDef(enum_def) => {
                let variant_names: Vec<&str> = enum_def.variants.iter().map(|v| v.name.as_str()).collect();
                println!("  🔷 Registered Enum: '{}' (variants: {})", enum_def.name, variant_names.join(", "));
                // Enum 定義をトランスパイル出力に含める
                rust_bundle.push_str(&transpile_enum(enum_def, TargetLanguage::Rust));
                rust_bundle.push_str("\n\n");
                go_bundle.push_str(&transpile_enum(enum_def, TargetLanguage::Go));
                go_bundle.push_str("\n\n");
                ts_bundle.push_str(&transpile_enum(enum_def, TargetLanguage::TypeScript));
                ts_bundle.push_str("\n\n");
            }

            // --- トレイト定義 ---
            Item::TraitDef(trait_def) => {
                let method_names: Vec<&str> = trait_def.methods.iter().map(|m| m.name.as_str()).collect();
                let law_names: Vec<&str> = trait_def.laws.iter().map(|(n, _)| n.as_str()).collect();
                println!("  📜 Registered Trait: '{}' (methods: {}, laws: {})",
                    trait_def.name, method_names.join(", "), law_names.join(", "));
            }

            // --- トレイト実装 ---
            Item::ImplDef(impl_def) => {
                println!("  🔧 Registered Impl: {} for {}", impl_def.trait_name, impl_def.target_type);
            }

            // --- Atom の処理 ---
            Item::Atom(atom) => {
                atom_count += 1;
                println!("  ✨ [1/4] Polishing Syntax: Atom '{}' identified.", atom.name);

                // --- 2. Verification (形式検証: Z3 + StdLib) ---
                // インポートされた atom は検証済み（契約のみ信頼）なのでスキップ
                if module_env.is_verified(&atom.name) {
                    println!("  ⚖️  [2/4] Verification: Skipped (imported, contract-trusted).");
                } else {
                    match verification::verify(atom, output_dir, &module_env) {
                        Ok(_) => {
                            println!("  ⚖️  [2/4] Verification: Passed. Logic verified with Z3.");
                            module_env.mark_verified(&atom.name);
                        },
                        Err(e) => {
                            eprintln!("  ❌ [2/4] Verification: Failed! Flaw detected: {}", e);
                            std::process::exit(1);
                        }
                    }
                }

                // --- 3. Codegen (LLVM 18 + Floating Point) ---
                // 各 Atom ごとに .ll ファイルを生成（またはモジュールを統合する拡張も可能）
                let atom_output_path = output_dir.join(format!("{}_{}", file_stem, atom.name));
                match codegen::compile(atom, &atom_output_path, &module_env) {
                    Ok(_) => println!("  ⚙️  [3/4] Tempering: Done. Compiled '{}' to LLVM IR.", atom.name),
                    Err(e) => {
                        eprintln!("  ❌ [3/4] Tempering: Failed! Codegen error: {}", e);
                        std::process::exit(1);
                    }
                }

                // --- 4. Transpile (多言語エクスポート) ---
                // バンドル用に各言語のコードを生成
                rust_bundle.push_str(&transpile(atom, TargetLanguage::Rust));
                rust_bundle.push_str("\n\n");

                go_bundle.push_str(&transpile(atom, TargetLanguage::Go));
                go_bundle.push_str("\n\n");

                ts_bundle.push_str(&transpile(atom, TargetLanguage::TypeScript));
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
