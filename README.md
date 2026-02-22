# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language (prototype).**

**Mumei (無銘)** は、ソースコードを

> parse → verify (Z3) → codegen (LLVM IR) → transpile (Rust / Go / TypeScript)

のパイプラインで処理し、形式検証に通った Atom を LLVM IR と各言語のソースコードへ出力する実験的な言語です。

---

## ✨ Type System 2.0（Refinement Types + f64/u64）

Mumei は **Refinement Types（精緻型）** をサポートします。

```mumei
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;
```

- `type Name = Base where predicate;` 形式
- `Base` は現在 `i64 | u64 | f64`
- `predicate` は Z3 で検証され、`atom` の引数に型注釈を付けると自動的に制約が適用されます

### 例: 型注釈で前提を削る

```mumei
type NonZero = i64 where v != 0;

atom safe_divide(a: i64, b: NonZero)
requires:
    true; // b != 0 は型(NonZero)が保証
ensures:
    true;
body: {
    a / b
};
```

---

## 📦 Standard Library（現在サポートされている呼び出し）

式として以下の関数呼び出しをサポートします：

- `sqrt(x)`
- `len(a)`
- `cast_to_int(x)`

注意：現状 `len()` は検証側で `arr_len` というシンボリック定数として扱われ、LLVM 側はダミー実装になっています（プロトタイプ段階）。

---

## 🛠️ Forging Process

1. **Polishing (Parser)**: `type` と `atom` をモジュール単位で解析。`if/else`、`let`、`while invariant`、関数呼び出し、配列アクセスをサポート。
2. **Verification (Z3)**: requires/ensures/loop invariant を検証。引数の精緻型制約を自動注入し、配列アクセスには境界チェックを挿入。
3. **Tempering (LLVM IR)**: Atom ごとに `.ll` を出力。
4. **Sharpening (Transpiler)**: 全 Atom をバンドルして `.rs/.go/.ts` を出力。

---

## 🚀 Quickstart（macOS）

### 1) 依存のインストール

```bash
xcode-select --install
brew install llvm@18 z3
```

### 2) ビルド & 実行

```bash
./build_and_run.sh

# 必要ならクリーンビルド
./build_and_run.sh --clean
```

`build_and_run.sh` が LLVM/Z3 の環境変数設定、ビルド、テスト用 `sword_test.mm` 生成、実行まで行います。

---

## 📄 Language Example（`sword_test.mm`）

```mumei
// Type System 2.0: Refinement Types
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;

// Atom 1: i64 ループ（loop invariant 検証）
atom sword_sum(n: Nat)
requires:
    n >= 0;
ensures:
    result >= 0;
body: {
    let s = 0;
    let i = 0;
    while i < n
    invariant: s >= 0 && i <= n
    {
        s = s + i;
        i = i + 1;
    };
    s
};

// Atom 2: f64 精緻型（浮動小数点の検証）
atom scale(x: Pos)
requires:
    x > 0.0;
ensures:
    result > 0.0;
body: {
    x * 2.0
};
```

---

## 📦 Outputs

`--output dist/katana` の場合：

- LLVM IR: `dist/katana_<AtomName>.ll`（Atom ごと）
- Rust: `dist/katana.rs`
- Go: `dist/katana.go`
- TypeScript: `dist/katana.ts`

---

## 📂 Project Structure

- `src/parser.rs`: AST / tokenizer / parser（`Expr::Float`, `Expr::Call` などを含む）
- `src/verification.rs`: Z3 による検証、精緻型の登録（グローバル型環境）
- `src/codegen.rs`: LLVM IR 生成（float/int 混在の昇格を含む）
- `src/transpiler/`: Rust/Go/TS への変換
- `src/main.rs`: コンパイラのオーケストレーター（Atom 単位の `.ll` 出力、言語別コードのバンドル出力）

---

## 🗺️ Roadmap

- [x] Refinement Types（Z3-backed）
- [x] `while` + loop invariant の検証
- [x] `f64` リテラル / `u64` ベース型の導入（基本制約のみ）
- [x] 標準関数呼び出し（`sqrt`, `len` など）
- [ ] Float 算術のより厳密な検証（現状は一部シンボリック扱い）
- [ ] 配列長モデルの実装（`len()` の実体化、境界チェックの強化）
- [ ] エディタ統合（LSP / VS Code Extension）

