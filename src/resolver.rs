//! # Resolver モジュール
//!
//! import 宣言を再帰的に処理し、依存モジュールの型・構造体・atom を
//! グローバル環境に登録する。循環参照の検出も行う。
//!
//! ## 設計方針
//! - Phase 1: ファイルベースの単純な import 解決
//! - Phase 2+: 完全修飾名（FQN）による名前空間分離、ModuleEnv ベースの管理
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use crate::parser::{self, Item};
use crate::verification::{self, MumeiError, MumeiResult};
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
/// items 内の Import 宣言を処理し、依存モジュールの定義をグローバル環境に登録する。
/// base_dir はインポート元ファイルの親ディレクトリ。
pub fn resolve_imports(items: &[Item], base_dir: &Path) -> MumeiResult<()> {
    let mut ctx = ResolverContext::new();
    resolve_imports_recursive(items, base_dir, &mut ctx)
}
/// 再帰的にインポートを解決する内部関数
fn resolve_imports_recursive(
    items: &[Item],
    base_dir: &Path,
    ctx: &mut ResolverContext,
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
            let imported_items = parser::parse_module(&source);
            let import_base_dir = resolved_path.parent().unwrap_or(Path::new("."));
            // 再帰的にインポートを解決（インポートされたモジュール内の import も処理）
            resolve_imports_recursive(&imported_items, import_base_dir, ctx)?;
            // インポートされたモジュールの定義をグローバル環境に登録
            let alias_prefix = import_decl.alias.as_deref();
            register_imported_items(&imported_items, alias_prefix)?;
            // ロード完了
            ctx.loading.remove(&resolved_path);
            ctx.loaded.insert(resolved_path, imported_items);
        }
    }
    Ok(())
}
/// インポートされたモジュールの Item をグローバル環境に登録する。
/// alias が指定されている場合、FQN（alias::name）でも登録する。
fn register_imported_items(items: &[Item], alias: Option<&str>) -> MumeiResult<()> {
    for item in items {
        match item {
            Item::TypeDef(refined_type) => {
                verification::register_type(refined_type)?;
                if let Some(prefix) = alias {
                    let mut fqn_type = refined_type.clone();
                    fqn_type.name = format!("{}::{}", prefix, refined_type.name);
                    verification::register_type(&fqn_type)?;
                }
            }
            Item::StructDef(struct_def) => {
                verification::register_struct(struct_def)?;
                if let Some(prefix) = alias {
                    let mut fqn_struct = struct_def.clone();
                    fqn_struct.name = format!("{}::{}", prefix, struct_def.name);
                    verification::register_struct(&fqn_struct)?;
                }
            }
            Item::Atom(atom) => {
                verification::register_atom(atom)?;
                if let Some(prefix) = alias {
                    let mut fqn_atom = atom.clone();
                    fqn_atom.name = format!("{}::{}", prefix, atom.name);
                    verification::register_atom(&fqn_atom)?;
                }
            }
            Item::Import(_) => {
                // 再帰的に処理済み
            }
        }
    }
    Ok(())
}
/// インポートパスを絶対パスに解決する。
/// 拡張子 .mm が省略されている場合は自動補完する。
fn resolve_path(import_path: &str, base_dir: &Path) -> MumeiResult<PathBuf> {
    let mut path = PathBuf::from(import_path);
    if path.extension().is_none() {
        path.set_extension("mm");
    }
    let resolved = if path.is_relative() {
        base_dir.join(&path)
    } else {
        path
    };
    let canonical = resolved.canonicalize().map_err(|e| {
        MumeiError::VerificationError(
            format!("Cannot resolve import path '{}' (base: '{}'): {}", import_path, base_dir.display(), e)
        )
    })?;
    Ok(canonical)
}
