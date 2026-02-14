# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language for AI Agents.**

「無銘（Mumei）」は、作者の個性を排し、数学的な「正しさ」のみを追求するAIネイティブなプログラミング言語です。AIがコードを生成する際、実行前にその論理的欠陥を数学的に証明・排除し、不純物のない「真実のコード」のみをマシンコード（LLVM）へと昇華させます。

---

## 🛠️ 設計思想 (Design Philosophy)

Mumeiは以下の3つの工程（鍛造プロセス）を経て、実行バイナリを生成します。

1. **Polishing (Parser):** `atom` と呼ばれる極小の関数単位でコードを解析します。
2. **The Ritual of Truth (Verification):** Z3 SMT Solverを用い、事前条件 (`requires`) が実装 (`body`) の安全性を数学的に担保しているか、反例がないかを検証します。
3. **Tempering (Codegen):** 検証を通過した清浄なコードのみを LLVM IR へと変換し、ネイティブな切れ味（実行速度）を与えます。

---

## 🚀 セットアップ (Installation)

Mumeiを鍛えるには、以下の霊器（ツール）が必要です。

### 1. 依存ライブラリの導入

* **LLVM 15:** ネイティブコード生成用
* **Z3 Solver:** 論理検証用

```bash
# macOS
brew install llvm@15 z3

# Ubuntu
sudo apt install llvm-15-dev libz3-dev

```

### 2. ビルド

```bash
git clone https://github.com/your-username/mumei.git
cd mumei
cargo build --release

```

---

## 📖 使い方 (Usage)

### 1. Mumeiコード (`.mm`) の記述

`sword_test.mm` を作成します。論理的に不備があるコードは、鍛造プロセス（コンパイル）を通過できません。

```rust
// 0除算の可能性を数学的に排除した「安全な割り算」
atom divide_ritual(a, b) {
    requires: b != 0;
    ensures: result * b == a;
    body: a / b;
}

```

### 2. 鍛造 (コンパイル)

```bash
# 検証を行い、LLVM IR (.ll) を出力します
cargo run -- sword_test.mm --output katana

```

### 3. 実行

生成された `.ll` ファイルは、LLVMのインタープリタや `clang` で実行可能です。

```bash
lli katana.ll
# またはバイナリ化
clang katana.ll -o katana_bin

```

---

## 📂 プロジェクト構造

* `src/parser.rs`: `pest` を使用した Mumei 構文解析器。
* `src/verification.rs`: `z3-rs` を介して、事前条件と実装の整合性を証明。
* `src/codegen.rs`: `inkwell` を使用し、検証済み AST から LLVM IR を生成。
* `grammar.pest`: Mumei の文法定義ファイル。

---

## 🗺️ ロードマップ (Roadmap)

* [ ] **Mumei Visualizer:** 検証プロセスの可視化。
* [ ] **Mumei Transpiler:** 検証済みコードを Rust/C++ へ変換。
* [ ] **Self-Healing Loop:** 検証に失敗した際、AIが自動で論理を修正する機能。
