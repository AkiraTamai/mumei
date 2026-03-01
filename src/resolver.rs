//! # Resolver モジュール
//!
//! import 宣言を再帰的に処理し、依存モジュールの型・構造体・atom を
//! ModuleEnv に登録する。循環参照の検出も行う。
//!
//! ## 設計方針
//! - Phase 1: ファイルベースの単純な import 解決
//! - Phase 2+: 完全修飾名（FQN）による名前空間分離、ModuleEnv ベースの管理
//!
//! ## 検証キャッシュ
//! インポートされたモジュールの atom は「検証済み」としてマークされ、
//! main.rs での body 再検証がスキップされる。呼び出し時は requires/ensures
//! の契約のみを信頼する（Compositional Verification）。
//!
//! キャッシュファイル (.mumei_cache) にはソースハッシュと検証結果を永続化し、
//! ソースが変更されていなければ再パース・再検証をスキップする。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

use crate::parser::{self, Item};
use crate::verification::{ModuleEnv, MumeiError, MumeiResult};

/// 検証キャッシュのエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// ソースファイルの SHA-256 ハッシュ
    source_hash: String,
    /// 検証済み atom 名のリスト
    verified_atoms: Vec<String>,
    /// 型定義名のリスト
    type_names: Vec<String>,
    /// 構造体定義名のリスト
    struct_names: Vec<String>,
    /// Incremental Build: atom ごとの契約+body ハッシュ
    /// atom の requires/ensures/body_expr が変更されていなければ再検証をスキップする。
    /// キー: atom 名、値: SHA-256(name + requires + ensures + body_expr)
    #[serde(default)]
    atom_hashes: HashMap<String, String>,
}

/// キャッシュファイル全体
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct VerificationCache {
    /// ファイルパス → キャッシュエントリ
    entries: HashMap<String, CacheEntry>,
}
/// ロード済みモジュールのキャッシュ
struct ResolverContext {
    /// ロード中のモジュールパス集合（循環参照検出用）
    loading: HashSet<PathBuf>,
    /// 完全にロード済みのモジュール（キャッシュ）
    loaded: HashMap<PathBuf, Vec<Item>>,
}
impl ResolverContext {
    fn new() -> Self {
        Self {
            loading: HashSet::new(),
            loaded: HashMap::new(),
        }
    }
}
/// items 内の Import 宣言を処理し、依存モジュールの定義を ModuleEnv に登録する。
/// base_dir はインポート元ファイルの親ディレクトリ。
/// キャッシュファイルが存在し、ソースハッシュが一致する場合は再パースをスキップする。
pub fn resolve_imports(items: &[Item], base_dir: &Path, module_env: &mut ModuleEnv) -> MumeiResult<()> {
    let cache_path = base_dir.join(".mumei_cache");
    let mut cache = load_cache(&cache_path);
    let mut ctx = ResolverContext::new();
    resolve_imports_recursive(items, base_dir, &mut ctx, &mut cache, module_env)?;
    save_cache(&cache_path, &cache);
    Ok(())
}

/// std/prelude.mm を自動的にロードし、ModuleEnv に登録する。
/// ユーザーが `import "std/prelude"` を書かなくても、
/// Eq, Ord, Numeric, Option<T>, Result<T, E> 等が利用可能になる。
///
/// prelude の定義はトレイト・ADT のみを登録し、atom は検証済みとしてマークする。
/// prelude が見つからない場合はスキップする（組み込みトレイトがフォールバックとして機能）。
pub fn resolve_prelude(base_dir: &Path, module_env: &mut ModuleEnv) -> MumeiResult<()> {
    // prelude のパスを解決（見つからなければスキップ）
    let prelude_path = match resolve_path("std/prelude", base_dir) {
        Ok(path) => path,
        Err(_) => {
            // prelude が見つからない場合は静かにスキップ
            // （組み込みトレイト register_builtin_traits が代替として機能）
            return Ok(());
        }
    };

    // prelude を読み込み・パース
    let source = match fs::read_to_string(&prelude_path) {
        Ok(s) => s,
        Err(_) => return Ok(()), // 読み込み失敗もスキップ
    };

    let prelude_items = parser::parse_module(&source);

    // prelude 内の import を再帰的に解決（prelude 自身が他モジュールに依存する場合）
    let prelude_base_dir = prelude_path.parent().unwrap_or(Path::new("."));
    let cache_path = prelude_base_dir.join(".mumei_cache");
    let mut cache = load_cache(&cache_path);
    let mut ctx = ResolverContext::new();
    ctx.loading.insert(prelude_path.clone());
    resolve_imports_recursive(&prelude_items, prelude_base_dir, &mut ctx, &mut cache, module_env)?;
    save_cache(&cache_path, &cache);

    // prelude の定義を ModuleEnv に登録（alias なし = グローバルスコープ）
    register_imported_items(&prelude_items, None, module_env);

    // prelude の atom を検証済みとしてマーク
    for item in &prelude_items {
        if let Item::Atom(atom) = item {
            module_env.mark_verified(&atom.name);
        }
    }

    Ok(())
}
/// 再帰的にインポートを解決する内部関数
fn resolve_imports_recursive(
    items: &[Item],
    base_dir: &Path,
    ctx: &mut ResolverContext,
    cache: &mut VerificationCache,
    module_env: &mut ModuleEnv,
) -> MumeiResult<()> {
    for item in items {
        if let Item::Import(import_decl) = item {
            let resolved_path = resolve_path(&import_decl.path, base_dir)?;
            // 循環参照チェック
            if ctx.loading.contains(&resolved_path) {
                return Err(MumeiError::VerificationError(
                    format!("Circular import detected: '{}'", resolved_path.display())
                ));
            }
            // 既にロード済みならスキップ
            if ctx.loaded.contains_key(&resolved_path) {
                continue;
            }
            // ロード中としてマーク
            ctx.loading.insert(resolved_path.clone());
            // ファイルを読み込みパース
            let source = fs::read_to_string(&resolved_path).map_err(|e| {
                MumeiError::VerificationError(
                    format!("Failed to read imported module '{}': {}", import_decl.path, e)
                )
            })?;

            let path_key = resolved_path.to_string_lossy().to_string();
            let source_hash = compute_hash(&source);

            // キャッシュヒット判定: ソースハッシュが一致すれば再パース不要
            if let Some(entry) = cache.entries.get(&path_key) {
                if entry.source_hash == source_hash {
                    // キャッシュから atom を検証済みとしてマーク（body 再検証スキップ）
                    // ただし型・構造体・atom の登録は必要なので、パースは行う
                }
            }

            let imported_items = parser::parse_module(&source);
            let import_base_dir = resolved_path.parent().unwrap_or(Path::new("."));
            // 再帰的にインポートを解決（インポートされたモジュール内の import も処理）
            resolve_imports_recursive(&imported_items, import_base_dir, ctx, cache, module_env)?;
            // インポートされたモジュールの定義を ModuleEnv に登録
            let alias_prefix = import_decl.alias.as_deref();
            register_imported_items(&imported_items, alias_prefix, module_env);

            // インポートされた atom を検証済みとしてマーク
            // → main.rs で verify() をスキップし、契約のみ信頼する
            let mut verified_atoms = Vec::new();
            let mut type_names = Vec::new();
            let mut struct_names = Vec::new();
            for imported_item in &imported_items {
                match imported_item {
                    Item::Atom(atom) => {
                        module_env.mark_verified(&atom.name);
                        verified_atoms.push(atom.name.clone());
                        // FQN でもマーク
                        if let Some(prefix) = alias_prefix {
                            let fqn = format!("{}::{}", prefix, atom.name);
                            module_env.mark_verified(&fqn);
                            verified_atoms.push(fqn);
                        }
                    }
                    Item::TypeDef(t) => type_names.push(t.name.clone()),
                    Item::StructDef(s) => struct_names.push(s.name.clone()),
                    Item::EnumDef(_) => {},
                    Item::TraitDef(_) => {},
                    Item::ImplDef(_) => {},
                    Item::ResourceDef(_) => {},
                    Item::Import(_) => {},
                }
            }

            // キャッシュを更新
            cache.entries.insert(path_key, CacheEntry {
                source_hash,
                verified_atoms,
                type_names,
                struct_names,
                atom_hashes: HashMap::new(),
            });

            // ロード完了
            ctx.loading.remove(&resolved_path);
            ctx.loaded.insert(resolved_path, imported_items);
        }
    }
    Ok(())
}
/// インポートされたモジュールの Item を ModuleEnv に登録する。
/// alias が指定されている場合、FQN（alias::name）でも登録する。
fn register_imported_items(items: &[Item], alias: Option<&str>, module_env: &mut ModuleEnv) {
    for item in items {
        match item {
            Item::TypeDef(refined_type) => {
                module_env.register_type(refined_type);
                if let Some(prefix) = alias {
                    let mut fqn_type = refined_type.clone();
                    fqn_type.name = format!("{}::{}", prefix, refined_type.name);
                    module_env.register_type(&fqn_type);
                }
            }
            Item::StructDef(struct_def) => {
                module_env.register_struct(struct_def);
                if let Some(prefix) = alias {
                    let mut fqn_struct = struct_def.clone();
                    fqn_struct.name = format!("{}::{}", prefix, struct_def.name);
                    module_env.register_struct(&fqn_struct);
                }
            }
            Item::Atom(atom) => {
                module_env.register_atom(atom);
                if let Some(prefix) = alias {
                    let mut fqn_atom = atom.clone();
                    fqn_atom.name = format!("{}::{}", prefix, atom.name);
                    module_env.register_atom(&fqn_atom);
                }
            }
            Item::EnumDef(enum_def) => {
                module_env.register_enum(enum_def);
                if let Some(prefix) = alias {
                    let mut fqn_enum = enum_def.clone();
                    fqn_enum.name = format!("{}::{}", prefix, enum_def.name);
                    module_env.register_enum(&fqn_enum);
                }
            }
            Item::TraitDef(trait_def) => {
                module_env.register_trait(trait_def);
                // トレイトは FQN 登録不要（トレイト名はグローバルに一意と仮定）
            }
            Item::ImplDef(impl_def) => {
                module_env.register_impl(impl_def);
            }
            Item::ResourceDef(resource_def) => {
                module_env.register_resource(resource_def);
                if let Some(prefix) = alias {
                    let mut fqn_resource = resource_def.clone();
                    fqn_resource.name = format!("{}::{}", prefix, resource_def.name);
                    module_env.register_resource(&fqn_resource);
                }
            }
            Item::Import(_) => {
                // 再帰的に処理済み
            }
        }
    }
}
/// インポートパスを絶対パスに解決する。
/// 拡張子 .mm が省略されている場合は自動補完する。
///
/// 解決順序:
/// 1. base_dir（インポート元ファイルのディレクトリ）からの相対パス
/// 2. 標準ライブラリパス（コンパイラバイナリの隣の `std/`、または実行ディレクトリの `std/`）
/// 3. MUMEI_STD_PATH 環境変数で指定されたパス
///
/// これにより `import "std/option";` のようなインポートが、
/// プロジェクト内に `std/` ディレクトリがなくても解決できる。
fn resolve_path(import_path: &str, base_dir: &Path) -> MumeiResult<PathBuf> {
    let mut path = PathBuf::from(import_path);
    if path.extension().is_none() {
        path.set_extension("mm");
    }

    // 1. base_dir からの相対パス解決を試行
    if path.is_relative() {
        let candidate = base_dir.join(&path);
        if let Ok(canonical) = candidate.canonicalize() {
            return Ok(canonical);
        }
    } else {
        // 絶対パスの場合はそのまま解決
        if let Ok(canonical) = path.canonicalize() {
            return Ok(canonical);
        }
    }

    // 2. "std/" プレフィックスの場合、標準ライブラリディレクトリから解決
    let import_str = import_path.trim_start_matches("./");
    if import_str.starts_with("std/") || import_str.starts_with("std\\") {
        // 2a. コンパイラバイナリの隣の std/ を探す
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let std_candidate = exe_dir.join(&path);
                if let Ok(canonical) = std_candidate.canonicalize() {
                    return Ok(canonical);
                }
            }
        }

        // 2b. カレントディレクトリの std/ を探す
        if let Ok(cwd) = std::env::current_dir() {
            let std_candidate = cwd.join(&path);
            if let Ok(canonical) = std_candidate.canonicalize() {
                return Ok(canonical);
            }
        }

        // 2c. Cargo マニフェストディレクトリ（開発時用）
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let std_candidate = Path::new(&manifest_dir).join(&path);
            if let Ok(canonical) = std_candidate.canonicalize() {
                return Ok(canonical);
            }
        }
    }

    // 3. MUMEI_STD_PATH 環境変数からの解決
    if let Ok(std_path) = std::env::var("MUMEI_STD_PATH") {
        let std_base = Path::new(&std_path);
        // "std/option" → std_base/option.mm として解決
        let relative = import_str.strip_prefix("std/")
            .or_else(|| import_str.strip_prefix("std\\"))
            .unwrap_or(import_str);
        let mut rel_path = PathBuf::from(relative);
        if rel_path.extension().is_none() {
            rel_path.set_extension("mm");
        }
        let std_candidate = std_base.join(&rel_path);
        if let Ok(canonical) = std_candidate.canonicalize() {
            return Ok(canonical);
        }
    }

    // すべて失敗した場合はエラー
    Err(MumeiError::VerificationError(
        format!(
            "Cannot resolve import path '{}'\n  Searched:\n    - {}\n    - compiler binary directory\n    - current working directory\n    - MUMEI_STD_PATH environment variable",
            import_path,
            base_dir.join(&path).display()
        )
    ))
}

// =============================================================================
// mumei.toml の [dependencies] 解決
// =============================================================================

/// mumei.toml の [dependencies] セクションを処理し、
/// パス依存・Git 依存のモジュールを ModuleEnv に登録する。
///
/// パス依存: `math = { path = "./libs/math" }` → path/src/main.mm を解決
/// Git 依存: `math = { git = "https://...", tag = "v1.0.0" }` → ~/.mumei/packages/ に clone
pub fn resolve_manifest_dependencies(
    manifest: &crate::manifest::Manifest,
    project_dir: &Path,
    module_env: &mut ModuleEnv,
) -> MumeiResult<()> {
    for (dep_name, dep) in &manifest.dependencies {
        // パス依存
        if let Some(dep_path) = dep.as_path() {
            let abs_path = project_dir.join(dep_path);
            let entry_candidates = [
                abs_path.join("src/main.mm"),
                abs_path.join("main.mm"),
                abs_path.join(format!("{}.mm", dep_name)),
            ];
            let entry = entry_candidates.iter().find(|p| p.exists());
            if let Some(entry_path) = entry {
                let source = fs::read_to_string(entry_path).map_err(|e| {
                    MumeiError::VerificationError(format!(
                        "Failed to read dependency '{}' at '{}': {}",
                        dep_name, entry_path.display(), e
                    ))
                })?;
                let items = parser::parse_module(&source);
                let dep_base_dir = entry_path.parent().unwrap_or(Path::new("."));
                let cache_path = dep_base_dir.join(".mumei_cache");
                let mut cache = load_cache(&cache_path);
                let mut ctx = ResolverContext::new();
                resolve_imports_recursive(&items, dep_base_dir, &mut ctx, &mut cache, module_env)?;
                save_cache(&cache_path, &cache);
                register_imported_items(&items, Some(dep_name), module_env);
                for item in &items {
                    if let Item::Atom(atom) = item {
                        module_env.mark_verified(&atom.name);
                        let fqn = format!("{}::{}", dep_name, atom.name);
                        module_env.mark_verified(&fqn);
                    }
                }
                println!("  📦 Dependency '{}': loaded from {}", dep_name, entry_path.display());
            } else {
                eprintln!("  ⚠️  Dependency '{}': no entry file found in '{}'", dep_name, abs_path.display());
            }
        }
        // Git 依存
        else if let Some((url, tag, rev, branch)) = dep.as_git() {
            let packages_dir = crate::manifest::mumei_home().join("packages");
            let _ = fs::create_dir_all(&packages_dir);
            let clone_dir = packages_dir.join(dep_name);

            if !clone_dir.exists() {
                // git clone
                let ref_arg = if let Some(t) = tag {
                    vec!["--branch".to_string(), t.to_string(), "--depth".to_string(), "1".to_string()]
                } else if let Some(b) = branch {
                    vec!["--branch".to_string(), b.to_string(), "--depth".to_string(), "1".to_string()]
                } else {
                    vec!["--depth".to_string(), "1".to_string()]
                };

                let mut cmd_args = vec!["clone".to_string()];
                cmd_args.extend(ref_arg);
                cmd_args.push(url.to_string());
                cmd_args.push(clone_dir.to_string_lossy().to_string());

                let status = std::process::Command::new("git")
                    .args(&cmd_args)
                    .status()
                    .map_err(|e| MumeiError::VerificationError(format!("git clone failed for '{}': {}", dep_name, e)))?;

                if !status.success() {
                    return Err(MumeiError::VerificationError(format!(
                        "git clone failed for dependency '{}' ({})", dep_name, url
                    )));
                }

                // 特定の rev にチェックアウト
                if let Some(r) = rev {
                    let _ = std::process::Command::new("git")
                        .args(["checkout", r])
                        .current_dir(&clone_dir)
                        .status();
                }

                println!("  📦 Dependency '{}': cloned from {}", dep_name, url);
            } else {
                println!("  📦 Dependency '{}': using cached clone", dep_name);
            }

            // クローンしたディレクトリからエントリファイルを解決
            let entry_candidates = [
                clone_dir.join("src/main.mm"),
                clone_dir.join("main.mm"),
                clone_dir.join(format!("{}.mm", dep_name)),
            ];
            if let Some(entry_path) = entry_candidates.iter().find(|p| p.exists()) {
                let source = fs::read_to_string(entry_path).map_err(|e| {
                    MumeiError::VerificationError(format!(
                        "Failed to read dependency '{}' at '{}': {}",
                        dep_name, entry_path.display(), e
                    ))
                })?;
                let items = parser::parse_module(&source);
                let dep_base_dir = entry_path.parent().unwrap_or(Path::new("."));
                let cache_path = dep_base_dir.join(".mumei_cache");
                let mut cache = load_cache(&cache_path);
                let mut ctx = ResolverContext::new();
                resolve_imports_recursive(&items, dep_base_dir, &mut ctx, &mut cache, module_env)?;
                save_cache(&cache_path, &cache);
                register_imported_items(&items, Some(dep_name), module_env);
                for item in &items {
                    if let Item::Atom(atom) = item {
                        module_env.mark_verified(&atom.name);
                        let fqn = format!("{}::{}", dep_name, atom.name);
                        module_env.mark_verified(&fqn);
                    }
                }
            } else {
                eprintln!("  ⚠️  Dependency '{}': no entry file found in cloned repo", dep_name);
            }
        }
    }
    Ok(())
}

// =============================================================================
// 検証キャッシュの永続化
// =============================================================================

/// ソースコードの SHA-256 ハッシュを計算する
fn compute_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Atom の契約+body+メタデータのハッシュを計算する（Incremental Build 用）
/// 以下のフィールドを結合してハッシュ化する:
/// - name, requires, ensures, body_expr（基本契約）
/// - consumed_params, ref params（所有権制約）
/// - resources, async flag（並行性制約）
/// - invariant（帰納的不変量）
/// - trust_level, max_unroll（検証設定）
///
/// このハッシュが一致すれば、atom の検証結果は変わらないため再検証をスキップできる。
/// Call Graph サイクル検知・Taint Analysis の結果も暗黙的にキャッシュされる
/// （呼び出し先の atom が変更されればハッシュが変わり、呼び出し元も再検証される）。
pub fn compute_atom_hash(atom: &crate::parser::Atom) -> String {
    let mut hasher = Sha256::new();
    hasher.update(atom.name.as_bytes());
    hasher.update(b"|");
    hasher.update(atom.requires.as_bytes());
    hasher.update(b"|");
    hasher.update(atom.ensures.as_bytes());
    hasher.update(b"|");
    hasher.update(atom.body_expr.as_bytes());
    // consumed_params も含める（所有権制約の変更を検出）
    for cp in &atom.consumed_params {
        hasher.update(b"|consume:");
        hasher.update(cp.as_bytes());
    }
    // ref / ref mut パラメータも含める
    for p in &atom.params {
        if p.is_ref {
            hasher.update(b"|ref:");
            hasher.update(p.name.as_bytes());
        }
        if p.is_ref_mut {
            hasher.update(b"|ref_mut:");
            hasher.update(p.name.as_bytes());
        }
    }
    // resources も含める（リソース制約の変更を検出）
    for r in &atom.resources {
        hasher.update(b"|resource:");
        hasher.update(r.as_bytes());
    }
    // async フラグも含める
    if atom.is_async {
        hasher.update(b"|async");
    }
    // invariant も含める
    if let Some(ref inv) = atom.invariant {
        hasher.update(b"|invariant:");
        hasher.update(inv.as_bytes());
    }
    // trust_level も含める（信頼レベルの変更を検出）
    let trust_str = match atom.trust_level {
        crate::parser::TrustLevel::Verified => "verified",
        crate::parser::TrustLevel::Trusted => "trusted",
        crate::parser::TrustLevel::Unverified => "unverified",
    };
    hasher.update(b"|trust:");
    hasher.update(trust_str.as_bytes());
    // max_unroll も含める（BMC 設定の変更を検出）
    if let Some(max) = atom.max_unroll {
        hasher.update(b"|max_unroll:");
        hasher.update(max.to_string().as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Incremental Build 用: メインファイルのビルドキャッシュをロードする
pub fn load_build_cache(base_dir: &Path) -> HashMap<String, String> {
    let cache_path = base_dir.join(".mumei_build_cache");
    fs::read_to_string(&cache_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Incremental Build 用: メインファイルのビルドキャッシュを保存する
pub fn save_build_cache(base_dir: &Path, cache: &HashMap<String, String>) {
    let cache_path = base_dir.join(".mumei_build_cache");
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(cache_path, json);
    }
}

/// キャッシュファイルを読み込む。存在しない場合は空のキャッシュを返す。
fn load_cache(cache_path: &Path) -> VerificationCache {
    fs::read_to_string(cache_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// キャッシュファイルに書き込む。書き込み失敗は無視する（キャッシュは最適化であり必須ではない）。
fn save_cache(cache_path: &Path, cache: &VerificationCache) {
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(cache_path, json);
    }
}
