mod ast;
mod parser;
mod verification;
mod codegen;
mod transpiler;
mod resolver;
#[allow(dead_code)]
mod manifest;
mod setup;
mod lsp;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use crate::transpiler::{TargetLanguage, transpile, transpile_enum, transpile_struct, transpile_trait, transpile_impl, transpile_module_header};
use crate::parser::{Item, ImportDecl};

// =============================================================================
// CLI: mumei build / verify / check / init / setup / doctor
// =============================================================================
//
// Usage:
//   mumei build input.mm -o dist/katana   # verify + codegen + transpile (default)
//   mumei verify input.mm                 # Z3 verification only
//   mumei check input.mm                  # parse + resolve + monomorphize (no Z3)
//   mumei init my_project                 # generate project template
//   mumei setup                           # download & configure Z3 + LLVM toolchain
//   mumei add <dep>                       # add dependency to mumei.toml
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
    /// Check development environment (Z3, LLVM, std library)
    Doctor,
    /// Download and configure Z3 + LLVM toolchain into ~/.mumei/
    Setup {
        /// Force re-download even if already installed
        #[arg(long)]
        force: bool,
    },
    /// Add a dependency to mumei.toml
    Add {
        /// Dependency specifier: local path (./path/to/lib) or package name
        dep: String,
    },
    /// Start Language Server Protocol server (stdio mode)
    Lsp,
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
        Some(Command::Doctor) => {
            cmd_doctor();
        }
        Some(Command::Setup { force }) => {
            setup::run(force);
        }
        Some(Command::Add { dep }) => {
            cmd_add(&dep);
        }
        Some(Command::Lsp) => {
            lsp::run();
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
                eprintln!("  setup   Download & configure Z3 + LLVM toolchain");
                eprintln!("  add     Add a dependency to mumei.toml");
                eprintln!("  lsp     Start Language Server Protocol server");
                eprintln!("  doctor  Check development environment");
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

    // mumei.toml の [dependencies] から依存パッケージを解決
    if let Some((proj_dir, m)) = manifest::find_and_load() {
        if let Err(e) = resolver::resolve_manifest_dependencies(&m, &proj_dir, &mut module_env) {
            eprintln!("  ⚠️  Dependency resolution warning: {}", e);
        }
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
            Item::ResourceDef(resource_def) => module_env.register_resource(resource_def),
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
            Item::Atom(a) => {
                atom_count += 1;
                let async_marker = if a.is_async { " (async)" } else { "" };
                let res_marker = if !a.resources.is_empty() {
                    format!(" [resources: {}]", a.resources.join(", "))
                } else { String::new() };
                println!("  ✨ Atom: '{}'{}{}", a.name, async_marker, res_marker);
            }
            Item::ResourceDef(r) => {
                let mode_str = match r.mode {
                    parser::ResourceMode::Exclusive => "exclusive",
                    parser::ResourceMode::Shared => "shared",
                };
                println!("  🔒 Resource: '{}' (priority={}, mode={})", r.name, r.priority, mode_str);
            }
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
    let input_path = Path::new(input);
    let base_dir = input_path.parent().unwrap_or(Path::new("."));
    let mut verified = 0;
    let mut failed = 0;
    let mut skipped = 0;

    // Incremental Build: ビルドキャッシュをロード
    let build_cache = resolver::load_build_cache(base_dir);
    let mut new_cache = std::collections::HashMap::new();

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
                    // Incremental Build: atom のハッシュを計算してキャッシュと比較
                    let atom_hash = resolver::compute_atom_hash(atom);
                    new_cache.insert(atom.name.clone(), atom_hash.clone());

                    if let Some(cached_hash) = build_cache.get(&atom.name) {
                        if *cached_hash == atom_hash {
                            println!("  ⚖️  '{}': skipped (unchanged, cached) ⏩", atom.name);
                            module_env.mark_verified(&atom.name);
                            skipped += 1;
                            continue;
                        }
                    }

                    match verification::verify(atom, output_dir, &module_env) {
                        Ok(_) => {
                            println!("  ⚖️  '{}': verified ✅", atom.name);
                            module_env.mark_verified(&atom.name);
                            verified += 1;
                        }
                        Err(e) => {
                            eprintln!("  ❌ '{}': verification failed: {}", atom.name, e);
                            // 検証失敗した atom はキャッシュから除外
                            new_cache.remove(&atom.name);
                            failed += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Incremental Build: キャッシュを保存
    resolver::save_build_cache(base_dir, &new_cache);

    println!("");
    if failed > 0 {
        eprintln!("❌ Verification: {} passed, {} failed, {} skipped (cached)", verified, failed, skipped);
        std::process::exit(1);
    }
    if skipped > 0 {
        println!("✅ Verification passed: {} verified, {} skipped (unchanged) ⚡", verified, skipped);
    } else {
        println!("✅ Verification passed: {} item(s) verified", verified);
    }
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
# authors = ["Your Name"]
# description = "A formally verified Mumei project"

[dependencies]
# 依存パッケージをここに記述
# example = {{ path = "./libs/example" }}
# math = {{ git = "https://github.com/user/math-mm", tag = "v1.0.0" }}

[build]
targets = ["rust", "go", "typescript"]
verify = true
max_unroll = 3

[proof]
cache = true
timeout_ms = 10000
"#, name);
    fs::write(project_dir.join("mumei.toml"), toml_content).unwrap();

    // src/main.mm — 充実したテンプレート（検証成功例 + 標準ライブラリ使用例）
    let main_content = format!(r#"// =============================================================
// {} — Mumei Project
// =============================================================
//
// このファイルは mumei init で生成されたサンプルプロジェクトです。
// 形式検証の基本的な使い方を示しています。
//
// 実行方法:
//   mumei build src/main.mm -o dist/output
//   mumei verify src/main.mm
//   mumei check src/main.mm

import "std/option" as option;

// --- 精緻型（Refinement Type） ---
// 型に述語制約を付与し、Z3 で自動検証します
type Nat = i64 where v >= 0;
type Pos = i64 where v > 0;

// --- 基本的な atom（関数） ---
// requires（事前条件）と ensures（事後条件）を Z3 が数学的に証明します
atom increment(n: Nat)
requires:
    n >= 0;
ensures:
    result >= 1;
body: {{
    n + 1
}};

// --- 複数パラメータ + 算術検証 ---
atom safe_add(a: Nat, b: Nat)
requires:
    a >= 0 && b >= 0;
ensures:
    result >= a && result >= b;
body: {{
    a + b
}};

// --- 条件分岐を含む検証 ---
atom clamp(value: i64, min_val: Nat, max_val: Pos)
requires:
    min_val >= 0 && max_val > 0 && min_val < max_val;
ensures:
    result >= min_val && result <= max_val;
body: {{
    if value < min_val then min_val
    else if value > max_val then max_val
    else value
}};

// --- スタック操作（契約による安全性保証） ---
atom stack_push(top: Nat, max_size: Pos)
requires:
    top >= 0 && max_size > 0 && top < max_size;
ensures:
    result >= 1 && result <= max_size;
body: {{
    top + 1
}};

atom stack_pop(top: Pos)
requires:
    top > 0;
ensures:
    result >= 0;
body: {{
    top - 1
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
    println!("  mumei doctor                            # check environment");
}

// =============================================================================
// mumei doctor — environment check
// =============================================================================

fn cmd_doctor() {
    use std::process::Command as Cmd;

    println!("🩺 Mumei Doctor: checking development environment...");
    println!();

    let mut ok_count = 0;
    let mut warn_count = 0;
    let mut fail_count = 0;

    // --- 1. Mumei compiler version ---
    println!("  Mumei compiler: v{}", env!("CARGO_PKG_VERSION"));
    ok_count += 1;

    // --- 2. Z3 solver ---
    match Cmd::new("z3").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim();
            if version.is_empty() {
                println!("  ⚠️  Z3: installed but version unknown");
                warn_count += 1;
            } else {
                println!("  ✅ Z3: {}", version);
                ok_count += 1;
            }
        }
        Err(_) => {
            println!("  ❌ Z3: not found");
            println!("     Install: brew install z3");
            fail_count += 1;
        }
    }

    // --- 3. LLVM ---
    let llvm_found = ["llc-18", "llc"].iter().any(|cmd| {
        Cmd::new(cmd).arg("--version").output().is_ok()
    });
    if llvm_found {
        // Try to get version
        let version_output = Cmd::new("llc-18").arg("--version").output()
            .or_else(|_| Cmd::new("llc").arg("--version").output());
        if let Ok(output) = version_output {
            let version = String::from_utf8_lossy(&output.stdout);
            let first_line = version.lines().next().unwrap_or("unknown");
            println!("  ✅ LLVM: {}", first_line.trim());
        } else {
            println!("  ✅ LLVM: installed");
        }
        ok_count += 1;
    } else {
        println!("  ❌ LLVM: not found");
        println!("     Install: brew install llvm@18");
        fail_count += 1;
    }

    // --- 4. Rust toolchain ---
    match Cmd::new("rustc").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  ✅ Rust: {}", version.trim());
            ok_count += 1;
        }
        Err(_) => {
            println!("  ⚠️  Rust: not found (optional, for generated .rs syntax check)");
            warn_count += 1;
        }
    }

    // --- 5. Go toolchain ---
    match Cmd::new("go").arg("version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  ✅ Go: {}", version.trim());
            ok_count += 1;
        }
        Err(_) => {
            println!("  ⚠️  Go: not found (optional, for generated .go compilation)");
            warn_count += 1;
        }
    }

    // --- 6. Node.js / TypeScript ---
    match Cmd::new("node").arg("--version").output() {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  ✅ Node.js: {}", version.trim());
            ok_count += 1;
        }
        Err(_) => {
            println!("  ⚠️  Node.js: not found (optional, for generated .ts execution)");
            warn_count += 1;
        }
    }

    // --- 7. std library ---
    // resolver と同じ探索順序: cwd → exe隣 → MUMEI_STD_PATH
    let std_modules = ["prelude.mm", "option.mm", "result.mm", "list.mm",
                       "stack.mm", "alloc.mm", "container/bounded_array.mm"];
    let mut std_base_dir: Option<std::path::PathBuf> = None;

    if Path::new("std/prelude.mm").exists() {
        std_base_dir = Some(std::path::PathBuf::from("std"));
    }
    if std_base_dir.is_none() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let candidate = exe_dir.join("std/prelude.mm");
                if candidate.exists() {
                    std_base_dir = Some(exe_dir.join("std"));
                }
            }
        }
    }
    if std_base_dir.is_none() {
        if let Ok(std_path) = std::env::var("MUMEI_STD_PATH") {
            let candidate = Path::new(&std_path).join("prelude.mm");
            if candidate.exists() {
                std_base_dir = Some(std::path::PathBuf::from(&std_path));
            }
        }
    }

    let mut std_found = 0;
    let mut std_missing = Vec::new();
    if let Some(ref base) = std_base_dir {
        for module in &std_modules {
            if base.join(module).exists() {
                std_found += 1;
            } else {
                std_missing.push(*module);
            }
        }
    } else {
        std_missing = std_modules.to_vec();
    }

    if std_missing.is_empty() {
        let location = std_base_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "?".to_string());
        println!("  ✅ std library: {}/{} modules found ({})", std_found, std_modules.len(), location);
        ok_count += 1;
    } else {
        let hint = if std_base_dir.is_none() {
            " (set MUMEI_STD_PATH or place std/ next to mumei binary)"
        } else { "" };
        println!("  ⚠️  std library: {}/{} modules found (missing: {}){}",
            std_found, std_modules.len(), std_missing.join(", "), hint);
        warn_count += 1;
    }

    // --- 8. mumei.toml (if in project directory) ---
    if Path::new("mumei.toml").exists() {
        // mumei.toml が見つかったらパースして内容を表示
        match manifest::load(Path::new("mumei.toml")) {
            Ok(m) => {
                println!("  ✅ mumei.toml: {} v{}", m.package.name, m.package.version);
                if !m.dependencies.is_empty() {
                    println!("     dependencies: {}", m.dependencies.keys()
                        .map(|k| k.as_str()).collect::<Vec<_>>().join(", "));
                }
                if !m.build.targets.is_empty() {
                    println!("     targets: {}", m.build.targets.join(", "));
                }
                ok_count += 1;
            }
            Err(e) => {
                println!("  ⚠️  mumei.toml: found but parse error: {}", e);
                warn_count += 1;
            }
        }
    } else {
        println!("  ℹ️  mumei.toml: not found (not in a Mumei project directory)");
    }

    // --- 9. ~/.mumei/ toolchain ---
    let mumei_home = manifest::mumei_home();
    let toolchains_dir = mumei_home.join("toolchains");
    if toolchains_dir.exists() {
        let mut tc_list = Vec::new();
        if let Ok(entries) = fs::read_dir(&toolchains_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        tc_list.push(name.to_string());
                    }
                }
            }
        }
        if tc_list.is_empty() {
            println!("  ℹ️  ~/.mumei/toolchains: empty (run `mumei setup`)");
        } else {
            tc_list.sort();
            println!("  ✅ ~/.mumei/toolchains: {}", tc_list.join(", "));
            ok_count += 1;
        }
    } else {
        println!("  ℹ️  ~/.mumei/toolchains: not found (run `mumei setup`)");
    }

    // --- Summary ---
    println!();
    if fail_count > 0 {
        println!("❌ Doctor: {} ok, {} warnings, {} errors", ok_count, warn_count, fail_count);
        println!("   Fix the errors above to use Mumei.");
        std::process::exit(1);
    } else if warn_count > 0 {
        println!("✅ Doctor: {} ok, {} warnings — Mumei is ready (optional tools missing)", ok_count, warn_count);
    } else {
        println!("✅ Doctor: {} ok — all tools available", ok_count);
    }
}

// =============================================================================
// mumei build — full pipeline (verify + codegen + transpile)
// =============================================================================

fn cmd_build(input: &str, output: &str) {
    println!("🗡️  Mumei: Forging the blade (Type System 2.0 + Generics enabled)...");

    // mumei.toml の自動検出と設定適用
    let manifest_config = manifest::find_and_load();
    let (build_cfg, proof_cfg) = if let Some((ref _proj_dir, ref m)) = manifest_config {
        println!("  📄 Using mumei.toml: {} v{}", m.package.name, m.package.version);
        (m.build.clone(), m.proof.clone())
    } else {
        (manifest::BuildConfig::default(), manifest::ProofConfig::default())
    };

    let (items, mut module_env, imports) = load_and_prepare(input);

    let output_path = Path::new(output);
    let output_dir = output_path.parent().unwrap_or(Path::new("."));
    let file_stem = output_path.file_stem().and_then(|s| s.to_str()).unwrap_or(output);
    let input_path = Path::new(input);
    let build_base_dir = input_path.parent().unwrap_or(Path::new("."));

    // Incremental Build: ビルドキャッシュをロード（proof.cache が false ならスキップ）
    let build_cache = if proof_cfg.cache {
        resolver::load_build_cache(build_base_dir)
    } else {
        std::collections::HashMap::new()
    };
    let mut build_cache_new = std::collections::HashMap::new();

    // [build] targets から有効なトランスパイル言語を決定
    let enable_rust = build_cfg.targets.iter().any(|t| t == "rust");
    let enable_go = build_cfg.targets.iter().any(|t| t == "go");
    let enable_ts = build_cfg.targets.iter().any(|t| t == "typescript" || t == "ts");
    let skip_verify = !build_cfg.verify;

    let mut atom_count = 0;

    // Transpiler バンドル初期化（有効な言語のみ）
    let mut rust_bundle = if enable_rust { transpile_module_header(&imports, file_stem, TargetLanguage::Rust) } else { String::new() };
    let mut go_bundle = if enable_go { transpile_module_header(&imports, file_stem, TargetLanguage::Go) } else { String::new() };
    let mut ts_bundle = if enable_ts { transpile_module_header(&imports, file_stem, TargetLanguage::TypeScript) } else { String::new() };

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
                // 構造体定義をトランスパイル出力に含める（有効な言語のみ）
                if enable_rust { rust_bundle.push_str(&transpile_struct(struct_def, TargetLanguage::Rust)); rust_bundle.push_str("\n\n"); }
                if enable_go { go_bundle.push_str(&transpile_struct(struct_def, TargetLanguage::Go)); go_bundle.push_str("\n\n"); }
                if enable_ts { ts_bundle.push_str(&transpile_struct(struct_def, TargetLanguage::TypeScript)); ts_bundle.push_str("\n\n"); }
            }

            // --- Enum 定義の登録 + トランスパイル ---
            Item::EnumDef(enum_def) => {
                let variant_names: Vec<&str> = enum_def.variants.iter().map(|v| v.name.as_str()).collect();
                println!("  🔷 Registered Enum: '{}' (variants: {})", enum_def.name, variant_names.join(", "));
                if enable_rust { rust_bundle.push_str(&transpile_enum(enum_def, TargetLanguage::Rust)); rust_bundle.push_str("\n\n"); }
                if enable_go { go_bundle.push_str(&transpile_enum(enum_def, TargetLanguage::Go)); go_bundle.push_str("\n\n"); }
                if enable_ts { ts_bundle.push_str(&transpile_enum(enum_def, TargetLanguage::TypeScript)); ts_bundle.push_str("\n\n"); }
            }

            // --- トレイト定義 + トランスパイル ---
            Item::TraitDef(trait_def) => {
                let method_names: Vec<&str> = trait_def.methods.iter().map(|m| m.name.as_str()).collect();
                let law_names: Vec<&str> = trait_def.laws.iter().map(|(n, _)| n.as_str()).collect();
                println!("  📜 Registered Trait: '{}' (methods: {}, laws: {})",
                    trait_def.name, method_names.join(", "), law_names.join(", "));
                if enable_rust { rust_bundle.push_str(&transpile_trait(trait_def, TargetLanguage::Rust)); rust_bundle.push_str("\n\n"); }
                if enable_go { go_bundle.push_str(&transpile_trait(trait_def, TargetLanguage::Go)); go_bundle.push_str("\n\n"); }
                if enable_ts { ts_bundle.push_str(&transpile_trait(trait_def, TargetLanguage::TypeScript)); ts_bundle.push_str("\n\n"); }
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
                // impl 定義をトランスパイル出力に含める（有効な言語のみ）
                if enable_rust { rust_bundle.push_str(&transpile_impl(impl_def, TargetLanguage::Rust)); rust_bundle.push_str("\n\n"); }
                if enable_go { go_bundle.push_str(&transpile_impl(impl_def, TargetLanguage::Go)); go_bundle.push_str("\n\n"); }
                if enable_ts { ts_bundle.push_str(&transpile_impl(impl_def, TargetLanguage::TypeScript)); ts_bundle.push_str("\n\n"); }
            }

            // --- リソース定義の登録 ---
            Item::ResourceDef(resource_def) => {
                let mode_str = match resource_def.mode {
                    parser::ResourceMode::Exclusive => "exclusive",
                    parser::ResourceMode::Shared => "shared",
                };
                println!("  🔒 Registered Resource: '{}' (priority={}, mode={})",
                    resource_def.name, resource_def.priority, mode_str);
            }

            // --- Atom の処理 ---
            Item::Atom(atom) => {
                atom_count += 1;
                let async_marker = if atom.is_async { " (async)" } else { "" };
                let res_marker = if !atom.resources.is_empty() {
                    format!(" [resources: {}]", atom.resources.join(", "))
                } else { String::new() };
                println!("  ✨ [1/4] Polishing Syntax: Atom '{}'{}{} identified.", atom.name, async_marker, res_marker);

                // --- 2. Verification (形式検証: Z3 + StdLib) ---
                if skip_verify {
                    println!("  ⚖️  [2/4] Verification: Skipped (verify=false in mumei.toml).");
                    module_env.mark_verified(&atom.name);
                } else if module_env.is_verified(&atom.name) {
                    // インポートされた atom は検証済み（契約のみ信頼）なのでスキップ
                    println!("  ⚖️  [2/4] Verification: Skipped (imported, contract-trusted).");
                } else {
                    // Incremental Build: atom ハッシュでキャッシュ比較
                    let atom_hash = resolver::compute_atom_hash(atom);
                    build_cache_new.insert(atom.name.clone(), atom_hash.clone());

                    let cache_hit = build_cache.get(&atom.name)
                        .map_or(false, |cached| *cached == atom_hash);

                    if cache_hit {
                        println!("  ⚖️  [2/4] Verification: Skipped (unchanged, cached) ⏩");
                        module_env.mark_verified(&atom.name);
                    } else {
                        match verification::verify_with_config(atom, output_dir, &module_env, proof_cfg.timeout_ms, build_cfg.max_unroll) {
                            Ok(_) => {
                                println!("  ⚖️  [2/4] Verification: Passed. Logic verified with Z3.");
                                module_env.mark_verified(&atom.name);
                            },
                            Err(e) => {
                                eprintln!("  ❌ [2/4] Verification: Failed! Flaw detected: {}", e);
                                build_cache_new.remove(&atom.name);
                                std::process::exit(1);
                            }
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
                // バンドル用に各言語のコードを生成（有効な言語のみ）
                if enable_rust { rust_bundle.push_str(&transpile(atom, TargetLanguage::Rust)); rust_bundle.push_str("\n\n"); }
                if enable_go { go_bundle.push_str(&transpile(atom, TargetLanguage::Go)); go_bundle.push_str("\n\n"); }
                if enable_ts { ts_bundle.push_str(&transpile(atom, TargetLanguage::TypeScript)); ts_bundle.push_str("\n\n"); }
            }
        }
    }

    // 各言語のファイルを一括書き出し（有効な言語のみ）
    if atom_count > 0 {
        println!("  🌍 [4/4] Sharpening: Exporting verified sources...");

        let mut created_files = Vec::new();
        let files: Vec<(&str, &str, bool)> = vec![
            (&rust_bundle, "rs", enable_rust),
            (&go_bundle, "go", enable_go),
            (&ts_bundle, "ts", enable_ts),
        ];

        for (code, ext, enabled) in files {
            if !enabled { continue; }
            let out_filename = format!("{}.{}", file_stem, ext);
            let out_full_path = output_dir.join(&out_filename);
            if let Err(e) = fs::write(&out_full_path, code) {
                eprintln!("  ❌ Failed to write {}: {}", out_filename, e);
                std::process::exit(1);
            }
            created_files.push(out_filename);
        }
        println!("  ✅ Done. Created: {}", created_files.join(", "));
        println!("🎉 Blade forged successfully with {} atoms.", atom_count);
    } else {
        println!("⚠️  Warning: No atoms found in the source file.");
    }

    // Incremental Build: ビルドキャッシュを保存
    resolver::save_build_cache(build_base_dir, &build_cache_new);
}

// =============================================================================
// mumei add — add dependency to mumei.toml
// =============================================================================

fn cmd_add(dep: &str) {
    // mumei.toml を探す
    let manifest_path = Path::new("mumei.toml");
    if !manifest_path.exists() {
        eprintln!("❌ Error: mumei.toml not found in current directory.");
        eprintln!("   Run `mumei init <project>` first, or cd into a Mumei project.");
        std::process::exit(1);
    }

    // 現在の mumei.toml を読み込み
    let content = fs::read_to_string(manifest_path).unwrap_or_else(|e| {
        eprintln!("❌ Error: Cannot read mumei.toml: {}", e);
        std::process::exit(1);
    });

    // パース確認
    if let Err(e) = manifest::load(manifest_path) {
        eprintln!("❌ Error: mumei.toml parse error: {}", e);
        std::process::exit(1);
    }

    // 依存の種類を判定
    let dep_entry = if dep.starts_with("./") || dep.starts_with("../") || dep.starts_with('/') {
        // ローカルパス依存
        let dep_path = Path::new(dep);
        if !dep_path.exists() {
            eprintln!("❌ Error: Path '{}' does not exist.", dep);
            std::process::exit(1);
        }
        // パッケージ名はディレクトリ名から推定
        let pkg_name = dep_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .replace('-', "_");
        let toml_line = format!("{} = {{ path = \"{}\" }}", pkg_name, dep);
        println!("📦 Adding local dependency: {} → {}", pkg_name, dep);
        (pkg_name, toml_line)
    } else if dep.contains("github.com") || dep.contains("gitlab.com") {
        // Git URL 依存
        let pkg_name = dep.split('/')
            .last()
            .unwrap_or("unknown")
            .trim_end_matches(".git")
            .replace('-', "_");
        let toml_line = format!("{} = {{ git = \"{}\" }}", pkg_name, dep);
        println!("📦 Adding git dependency: {} → {}", pkg_name, dep);
        (pkg_name, toml_line)
    } else {
        // パッケージ名のみ（レジストリ依存 — 将来対応）
        let toml_line = format!("{} = \"*\"", dep);
        println!("📦 Adding dependency: {} (registry lookup not yet implemented)", dep);
        (dep.to_string(), toml_line)
    };

    // mumei.toml に追記
    let new_content = if content.contains("[dependencies]") {
        // [dependencies] セクションが既にある場合、その直後に追記
        content.replace(
            "[dependencies]",
            &format!("[dependencies]\n{}", dep_entry.1),
        )
    } else {
        // [dependencies] セクションがない場合、末尾に追加
        format!("{}\n[dependencies]\n{}\n", content.trim_end(), dep_entry.1)
    };

    fs::write(manifest_path, new_content).unwrap_or_else(|e| {
        eprintln!("❌ Error: Cannot write mumei.toml: {}", e);
        std::process::exit(1);
    });

    println!("✅ Added '{}' to mumei.toml", dep_entry.0);
}

// end of src/main.rs
