// =============================================================
// std/prelude.mm — Mumei Standard Prelude
// =============================================================
// 全モジュールに自動インポートされる基盤定義。
// 基本トレイト（数学的保証付き）、基本 ADT、
// および将来の動的メモリ管理に向けた抽象インターフェースを提供する。
//
// このファイルはコンパイラが暗黙的にロードする。
// ユーザーが `import "std/prelude"` を書く必要はない。
//
// 注: 基本トレイト (Eq, Ord, Numeric) の impl は
//     コンパイラが i64/u64/f64 に対して自動適用する。
//     law（推移律など）は Z3 上で既知の公理として扱われる。

// =============================================================
// A. 基本トレイト（数学的基盤）
// =============================================================
// Mumei の law 機構により、単なるメソッド定義以上の
// 「数学的保証」が Z3 によって検証される。

// --- Eq: 等価性 ---
// 反射律・対称律を Z3 で保証する。
trait Eq {
    fn eq(a: Self, b: Self) -> bool;
    law reflexive: eq(x, x) == true;
    law symmetric: eq(a, b) => eq(b, a);
}

// --- Ord: 全順序 ---
// 反射律・推移律を Z3 で保証する。
// Eq を暗黙的に前提とする（将来のトレイト継承で明示化予定）。
trait Ord {
    fn leq(a: Self, b: Self) -> bool;
    law reflexive: leq(x, x) == true;
    law transitive: leq(a, b) && leq(b, c) => leq(a, c);
}

// --- Numeric: 算術演算 ---
// 加法の交換律を Z3 で保証する。
// div の第2引数に精緻型制約 `where v != 0` を付与し、
// ゼロ除算を型レベルで排除する。
// Z3 は多相的な演算においても常にゼロ除算の可能性をチェックする。
trait Numeric {
    fn add(a: Self, b: Self) -> Self;
    fn sub(a: Self, b: Self) -> Self;
    fn mul(a: Self, b: Self) -> Self;
    fn div(a: Self, b: Self where v != 0) -> Self;
    law commutative_add: add(a, b) == add(b, a);
}

// =============================================================
// B. 基本 ADT（ジェネリック列挙型・構造体）
// =============================================================

// --- Generic Pair ---
struct Pair<T, U> {
    first: T,
    second: U
}

// --- Generic Option ---
enum Option<T> {
    None,
    Some(T)
}

// --- Generic Result ---
enum Result<T, E> {
    Ok(T),
    Err(E)
}

// --- Generic List (recursive ADT) ---
enum List<T> {
    Nil,
    Cons(T, Self)
}

// =============================================================
// C. コレクション抽象インターフェース（ロードマップ）
// =============================================================
// 動的メモリ管理（alloc）導入前に「インターフェース」として
// 定義しておくことで、将来の実装差し替えを容易にする。
//
// 現時点では固定長配列ベースのコードでも、
// これらのトレイトに準拠して書いておけば、
// alloc 導入後に Vector<T> 等の具体実装に
// 差し替えるだけでロジックの変更が不要になる。

// --- Sequential: 順序付きコレクション ---
// Vector<T> の抽象インターフェース。
// law により長さの非負性を型レベルで保証する。
//
// 将来の alloc 導入時:
//   impl Sequential for Vector<T> { ... }
//   として具体実装を差し込む。
trait Sequential {
    fn seq_len(s: Self) -> i64;
    fn seq_get(s: Self, index: i64) -> i64;
    law non_negative_length: seq_len(x) >= 0;
}

// --- Hashable: ハッシュ可能な型 ---
// HashMap<K, V> の Key 制約として使用する。
// 決定性（同じ値は同じハッシュ）を Z3 で保証する。
//
// 将来:
//   atom lookup<K: Hashable + Eq, V>(map: HashMap<K, V>, key: K) -> Option<V>
//   のように型制約として活用する。
trait Hashable {
    fn hash(a: Self) -> i64;
    law deterministic: hash(x) == hash(x);
}

// =============================================================
// D. 動的メモリ管理（alloc）基盤
// =============================================================
// RawPtr 型、所有権トレイト、Vector 構造体を定義する。
// Z3 による線形性チェックと精緻型制約により、
// 二重解放・メモリリーク・バッファオーバーフローを
// コンパイル時に論理的に排除する。

// --- STEP 1: RawPtr — 生ポインタの精緻型表現 ---
// LLVM IR レベルでは i64 のラッパー。
// Z3 上ではシンボリック整数として扱い、
// null チェックや境界チェックを精緻型で保証する。
// 有効なポインタは >= 0、null は -1 で表現。
type RawPtr = i64 where v >= 0;
type NullablePtr = i64 where v >= -1;

// --- STEP 2: 所有権トレイト（Linear Types の近似）---
// Owned トレイト: リソースの生存状態を追跡する。
// law により「消費前は必ず生存している」ことを Z3 で保証。
//
// Z3 による線形性の検証:
//   各変数 x に対して is_alive(x) フラグを管理。
//   consume(x) 呼び出し後、is_alive(x) == false となる。
//   false になった変数へのアクセスは Z3 が Unsat を返し、
//   コンパイルエラーとなる。
//
// atom take_ownership(resource: T) consume resource;
//   → resource は以後使用不可（Z3 が追跡）
trait Owned {
    fn is_alive(a: Self) -> bool;
    fn consume(a: Self) -> Self;
    law alive_before_consume: is_alive(x) == true;
}

// --- STEP 3: Vector<T> 構造体定義 ---
// ヒープ上のメモリを管理する動的配列。
// 精緻型制約による不変条件（Z3 で常に検証）:
//   - ptr >= 0（有効なポインタ）
//   - len >= 0（要素数は非負）
//   - cap > 0（容量は正）
//   - len <= cap（暗黙: push の requires で保証）
struct Vector<T> {
    ptr: i64 where v >= 0,
    len: i64 where v >= 0,
    cap: i64 where v > 0
}

// --- Vector 操作 Atom ---

// メモリ確保: 指定サイズのヒープメモリを確保
// 失敗時は -1（null）を返す
atom alloc_raw(size: i64)
    requires: size > 0;
    ensures: result >= -1;
    body: {
        if size > 0 { 0 } else { -1 }
    }

// メモリ解放: 有効なポインタのみ受け付ける
atom dealloc_raw(ptr: i64)
    requires: ptr >= 0;
    ensures: result >= 0;
    body: { 0 }

// Vector 新規作成: 初期容量を指定して空の Vector を生成
atom vec_new(initial_cap: i64)
    requires: initial_cap > 0;
    ensures: result >= 0;
    body: { 0 }

// Vector push: len < cap の場合のみ許可（Z3 がコンパイル時に検証）
atom vec_push(vec_len: i64, vec_cap: i64)
    requires: vec_len >= 0 && vec_cap > 0 && vec_len < vec_cap;
    ensures: result >= 0 && result <= vec_cap;
    body: { vec_len + 1 }

// Vector get: 境界チェック付き（0 <= index < len）
atom vec_get(vec_len: i64, index: i64)
    requires: vec_len > 0 && index >= 0 && index < vec_len;
    ensures: result >= 0;
    body: { index }

// Vector 長さ取得
atom vec_len(len: i64)
    requires: len >= 0;
    ensures: result >= 0 && result == len;
    body: { len }

// Vector 空判定
atom vec_is_empty(len: i64)
    requires: len >= 0;
    ensures: result >= 0 && result <= 1;
    body: {
        if len == 0 { 1 } else { 0 }
    }

// Vector 容量拡張
atom vec_grow(old_cap: i64, new_cap: i64)
    requires: old_cap > 0 && new_cap > old_cap;
    ensures: result > old_cap;
    body: { new_cap }

// Vector 解放: メモリを解放し len を 0 にリセット
atom vec_drop(vec_len: i64, vec_ptr: i64)
    requires: vec_len >= 0 && vec_ptr >= 0;
    ensures: result >= 0;
    body: { 0 }

// 安全な push: 容量チェック付き（Result 型: 0=Ok, 1=Err）
atom vec_push_safe(vec_len: i64, vec_cap: i64)
    requires: vec_len >= 0 && vec_cap > 0;
    ensures: result >= 0 && result <= 1;
    body: {
        if vec_len < vec_cap { 0 } else { 1 }
    }

// --- HashMap ロードマップ（将来実装）---
// Hashable + Eq をキー制約として使用:
//
//   struct HashMap<K, V> {
//       buckets: i64 where v >= 0,
//       size: i64 where v >= 0,
//       capacity: i64 where v > 0
//   }
//
//   atom map_insert<K: Hashable + Eq, V>(m: HashMap<K, V>, key: K, val: V)
//       requires: m.size < m.capacity;
//       ensures: result.size <= m.size + 1;
//       body: { /* hash-based implementation */ };

// =============================================================
// E. Prelude Atoms（基本操作）
// =============================================================

// Option の判定: Some(tag=1) なら 1, None(tag=0) なら 0
atom prelude_is_some(opt: i64)
    requires: opt >= 0 && opt <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match opt {
            1 => 1,
            _ => 0
        }
    }

// Option の判定: None(tag=0) なら 1, Some(tag=1) なら 0
atom prelude_is_none(opt: i64)
    requires: opt >= 0 && opt <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match opt {
            0 => 1,
            _ => 0
        }
    }

// Result の判定: Ok(tag=0) なら 1, Err(tag=1) なら 0
atom prelude_is_ok(res: i64)
    requires: res >= 0 && res <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match res {
            0 => 1,
            _ => 0
        }
    }
