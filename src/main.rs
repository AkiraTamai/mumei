mod ast;
mod parser;
mod verification;
mod codegen;
mod transpiler;
mod resolver;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use crate::transpiler::{TargetLanguage, transpile, transpile_enum, transpile_struct, transpile_trait, transpile_impl, transpile_module_header};
use crate::parser::{Item, ImportDecl};

// =============================================================================
// CLI: mumei build / verify / check / init
// =============================================================================
//
// Usage:
//   mumei build input.mm -o dist/katana   # verify + codegen + transpile (default)
//   mumei verify input.mm                 # Z3 verification only
//   mumei check input.mm                  # parse + resolve + monomorphize (no Z3)
//   mumei init my_project                 # generate project template
//   mumei input.mm -o dist/katana         # backward compat → same as build

#[derive(Parser)]
#[command(
    name = "mumei",
    version = env!("CARGO_PKG_VERSION"),
    about = "🗡️ Mumei — Mathematical Proof-Driven Programming Language",
    long_about = "Formally verified language: parse → resolve → monomorphize → verify (Z3) → codegen (LLVM IR) → transpile (Rust/Go/TypeScript)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Input .mm file (backward compat: `mumei input.mm` = `mumei build input.mm`)
    #[arg(global = false)]
    input: Option<String>,

    /// Output base name (for .ll, .rs, .go, .ts)
    #[arg(short, long, default_value = "katana")]
    output: String,
}

#[derive(Subcommand)]
enum Command {
    /// Verify + compile to LLVM IR + transpile to Rust/Go/TypeScript (default)
    Build {
        /// Input .mm file
        input: String,
        /// Output base name
        #[arg(short, long, default_value = "katana")]
        output: String,
    },
    /// Z3 formal verification only (no codegen, no transpile)
    Verify {
        /// Input .mm file
        input: String,
    },
    /// Parse + resolve + monomorphize only (no Z3, fast syntax check)
    Check {
        /// Input .mm file
        input: String,
    },
    /// Generate a new Mumei project template
    Init {
        /// Project directory name
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Build { input, output }) => {
            cmd_build(&input, &output);
        }
        Some(Command::Verify { input }) => {
            cmd_verify(&input);
        }
        Some(Command::Check { input }) => {
            cmd_check(&input);
        }
        Some(Command::Init { name }) => {
            cmd_init(&name);
        }
        None => {
            // 後方互換: `mumei input.mm -o dist/katana` → build として実行
            if let Some(ref input) = cli.input {
                cmd_build(input, &cli.output);
            } else {
                eprintln!("Usage: mumei <COMMAND> or mumei <input.mm>");
                eprintln!("  build   Verify + compile + transpile (default)");
                eprintln!("  verify  Z3 formal verification only");
                eprintln!("  check   Parse + resolve only (fast syntax check)");
                eprintln!("  init    Generate a new project template");
                eprintln!("Run `mumei --help` for full usage.");
                std::process::exit(1);
            }
        }
    }
}

// =============================================================================
// Shared pipeline helpers
// =============================================================================

/// ソースファイルを読み込む
fn load_source(input: &str) -> String {
    fs::read_to_string(input).unwrap_or_else(|_| {
        eprintln!("❌ Error: Could not read Mumei source file '{}'", input);
        std::process::exit(1);
    })
}

/// parse → resolve → monomorphize → ModuleEnv に全定義を登録
fn load_and_prepare(input: &str) -> (Vec<Item>, verification::ModuleEnv, Vec<ImportDecl>) {
    let source = load_source(input);
    let items = parser::parse_module(&source);

    let mut module_env = verification::ModuleEnv::new();
    verification::register_builtin_traits(&mut module_env);
    let input_path = Path::new(input);
    let base_dir = input_path.parent().unwrap_or(Path::new("."));

    // std/prelude.mm の自動ロード（Eq, Ord, Numeric, Option<T>, Result<T, E> 等）
    // prelude が見つからない場合は組み込みトレイトがフォールバックとして機能する
    if let Err(e) = resolver::resolve_prelude(base_dir, &mut module_env) {
        eprintln!("  ⚠️  Prelude load warning: {}", e);
        // prelude のロード失敗は致命的ではない（組み込みトレイトが代替）
    }

    if let Err(e) = resolver::resolve_imports(&items, base_dir, &mut module_env) {
        eprintln!("  ❌ Import Resolution Failed: {}", e);
        std::process::exit(1);
    }

    let mut mono = ast::Monomorphizer::new();
    mono.collect(&items);
    let items = if mono.has_generics() {
        let mono_items = mono.monomorphize(&items);
        println!("  🔬 Monomorphization: {} generic instance(s) expanded.", mono.instances().len());
        mono_items
    } else {
        items
    };

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

    (items, module_env, imports)
}

// =============================================================================
// mumei check — parse + resolve + monomorphize only
// =============================================================================

fn cmd_check(input: &str) {
    println!("🗡️  Mumei check: parsing and resolving '{}'...", input);
    let (items, _module_env, _imports) = load_and_prepare(input);

    let mut type_count = 0;
    let mut struct_count = 0;
    let mut enum_count = 0;
    let mut trait_count = 0;
    let mut atom_count = 0;
    for item in &items {
        match item {
            Item::Import(decl) => {
                let alias_str = decl.alias.as_deref().unwrap_or("(none)");
                println!("  📦 Import: '{}' as '{}'", decl.path, alias_str);
            }
            Item::TypeDef(t) => { type_count += 1; println!("  ✨ Type: '{}' ({})", t.name, t._base_type); }
            Item::StructDef(s) => { struct_count += 1; println!("  🏗️  Struct: '{}'", s.name); }
            Item::EnumDef(e) => { enum_count += 1; println!("  🔷 Enum: '{}'", e.name); }
            Item::TraitDef(t) => { trait_count += 1; println!("  📜 Trait: '{}'", t.name); }
            Item::ImplDef(i) => { println!("  🔧 Impl: {} for {}", i.trait_name, i.target_type); }
            Item::Atom(a) => { atom_count += 1; println!("  ✨ Atom: '{}'", a.name); }
        }
    }
    println!("✅ Check passed: {} types, {} structs, {} enums, {} traits, {} atoms",
        type_count, struct_count, enum_count, trait_count, atom_count);
}

// =============================================================================
// mumei verify — Z3 verification only (no codegen, no transpile)
// =============================================================================

fn cmd_verify(input: &str) {
    println!("🗡️  Mumei verify: verifying '{}'...", input);
    let (items, mut module_env, _imports) = load_and_prepare(input);

    let output_dir = Path::new(".");
    let mut verified = 0;
    let mut failed = 0;

    for item in &items {
        match item {
            Item::ImplDef(impl_def) => {
                println!("  🔧 Verifying impl {} for {}...", impl_def.trait_name, impl_def.target_type);
                match verification::verify_impl(impl_def, &module_env) {
                    Ok(_) => {
                        println!("    ✅ Laws verified");
                        verified += 1;
                    }
                    Err(e) => {
                        eprintln!("    ❌ Law verification failed: {}", e);
                        failed += 1;
                    }
                }
            }
            Item::Atom(atom) => {
                if module_env.is_verified(&atom.name) {
                    println!("  ⚖️  '{}': skipped (imported, contract-trusted)", atom.name);
                } else {
                    match verification::verify(atom, output_dir, &module_env) {
                        Ok(_) => {
                            println!("  ⚖️  '{}': verified ✅", atom.name);
                            module_env.mark_verified(&atom.name);
                            verified += 1;
                        }
                        Err(e) => {
                            eprintln!("  ❌ '{}': verification failed: {}", atom.name, e);
                            failed += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    println!("");
    if failed > 0 {
        eprintln!("❌ Verification: {} passed, {} failed", verified, failed);
        std::process::exit(1);
    }
    println!("✅ Verification passed: {} item(s) verified", verified);
}

// =============================================================================
// mumei init — generate project template
// =============================================================================

fn cmd_init(name: &str) {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        eprintln!("❌ Error: Directory '{}' already exists", name);
        std::process::exit(1);
    }

    // ディレクトリ構造を作成
    fs::create_dir_all(project_dir.join("src")).unwrap_or_else(|e| {
        eprintln!("❌ Error: Failed to create directory: {}", e);
        std::process::exit(1);
    });

    // mumei.toml
    let toml_content = format!(r#"[package]
name = "{}"
version = "0.1.0"

[dependencies]
# 依存パッケージをここに記述
# example = {{ git = "https://github.com/user/example-mm", rev = "main" }}
"#, name);
    fs::write(project_dir.join("mumei.toml"), toml_content).unwrap();

    // src/main.mm
    let main_content = format!(r#"// =============================================================
// {} — Mumei Project
// =============================================================

import "std/option" as option;

type Nat = i64 where v >= 0;

atom hello(n: Nat)
requires:
    n >= 0;
ensures:
    result >= 0;
body: {{
    n + 1
}};
"#, name);
    fs::write(project_dir.join("src/main.mm"), main_content).unwrap();

    println!("🗡️  Created new Mumei project '{}'", name);
    println!("");
    println!("  {}/", name);
    println!("  ├── mumei.toml");
    println!("  └── src/");
    println!("      └── main.mm");
    println!("");
    println!("Get started:");
    println!("  cd {}", name);
    println!("  mumei build src/main.mm -o dist/output");
    println!("  mumei verify src/main.mm");
    println!("  mumei check src/main.mm");
}

// =============================================================================
// mumei build — full pipeline (verify + codegen + transpile)
// =============================================================================

fn cmd_build(input: &str, output: &str) {
    println!("🗡️  Mumei: Forging the blade (Type System 2.0 + Generics enabled)...");

    let (items, mut module_env, imports) = load_and_prepare(input);

    let output_path = Path::new(output);
    let output_dir = output_path.parent().unwrap_or(Path::new("."));
    let file_stem = output_path.file_stem().and_then(|s| s.to_str()).unwrap_or(output);

    let mut atom_count = 0;

    // Transpiler バンドル初期化
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

            // --- トレイト定義 + トランスパイル ---
            Item::TraitDef(trait_def) => {
                let method_names: Vec<&str> = trait_def.methods.iter().map(|m| m.name.as_str()).collect();
                let law_names: Vec<&str> = trait_def.laws.iter().map(|(n, _)| n.as_str()).collect();
                println!("  📜 Registered Trait: '{}' (methods: {}, laws: {})",
                    trait_def.name, method_names.join(", "), law_names.join(", "));
                // トレイト定義をトランスパイル出力に含める
                rust_bundle.push_str(&transpile_trait(trait_def, TargetLanguage::Rust));
                rust_bundle.push_str("\n\n");
                go_bundle.push_str(&transpile_trait(trait_def, TargetLanguage::Go));
                go_bundle.push_str("\n\n");
                ts_bundle.push_str(&transpile_trait(trait_def, TargetLanguage::TypeScript));
                ts_bundle.push_str("\n\n");
            }

            // --- トレイト実装の登録 + 法則検証 + トランスパイル ---
            Item::ImplDef(impl_def) => {
                println!("  🔧 Registered Impl: {} for {}", impl_def.trait_name, impl_def.target_type);
                // impl が trait の全 law を満たしているか Z3 で検証
                match verification::verify_impl(impl_def, &module_env) {
                    Ok(_) => println!("    ✅ Laws verified for impl {} for {}", impl_def.trait_name, impl_def.target_type),
                    Err(e) => {
                        eprintln!("    ❌ Law verification failed: {}", e);
                        std::process::exit(1);
                    }
                }
                // impl 定義をトランスパイル出力に含める
                rust_bundle.push_str(&transpile_impl(impl_def, TargetLanguage::Rust));
                rust_bundle.push_str("\n\n");
                go_bundle.push_str(&transpile_impl(impl_def, TargetLanguage::Go));
                go_bundle.push_str("\n\n");
                ts_bundle.push_str(&transpile_impl(impl_def, TargetLanguage::TypeScript));
                ts_bundle.push_str("\n\n");
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

// end of src/main.rs
