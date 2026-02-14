# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language for AI Agents.**

「無銘（Mumei）」は、作者の個性を排し、数学的な「正しさ」のみを追求するAIネイティブなプログラミング言語です。AIがコードを生成する際、実行前にその論理的欠陥を数学的に証明・排除し、不純物のない「真実のコード」のみをマシンコード（LLVM）へと昇華させます。

---

## 🛠️ 設計思想 (Design Philosophy)

Mumeiは以下の4つの工程（鍛造プロセス）を経て、実行バイナリと検証レポートを生成します。

1. **Polishing (Parser):** `atom` と呼ばれる極小の関数単位でコードを解析します。
2. **The Ritual of Truth (Verification):** Z3 SMT Solverを用い、事前条件 (`requires`) が実装 (`body`) の安全性を数学的に担保しているか検証します。
3. **Visual Inspection (Visualizer):** 検証で発見された「論理の亀裂（反例）」をリアルタイムで視覚化します。
4. **Tempering (Codegen):** 検証を通過した清浄なコードのみを LLVM IR へと変換し、ネイティブな切れ味（実行速度）を与えます。

---

## 🚀 セットアップ (Installation)

Mumeiを鍛えるには、以下のツールが必要です。

### 1. 依存ライブラリの導入

* **LLVM 15:** ネイティブコード生成用
* **Z3 Solver:** 論理検証用
* **Python 3.x:** ビジュアライザー起動用

```bash
# macOS
brew install llvm@15 z3

# Ubuntu
sudo apt install llvm-15-dev libz3-dev

# Python dependencies
pip install streamlit pandas

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

`sword_test.mm` を作成します。

```rust
// 0除算の可能性を数学的に排除した「安全な割り算」
atom divide_ritual(a, b) {
    requires: b != 0;
    ensures: result * b == a;
    body: a / b;
}

```

### 2. 鍛造 (コンパイルと検証)

```bash
# 検証を行い、LLVM IR (.ll) を出力します
# 同時に visualizer/report.json が生成されます
cargo run -- sword_test.mm --output katana

```

---

## 📊 Inspection (Visualizer)

Mumeiは単に「エラー」を返すだけでなく、**なぜその論理が破綻したのか**を具体的な数値で提示します。

1. **ビジュアライザーの起動:**
```bash
streamlit run visualizer/app.py

```


2. **機能:**
* 検証ステータスのリアルタイム表示。
* 検証失敗時の **反例（Counter-example）** の提示（例：b=0 のときエラーになる、など）。
* AI向けの修正アドバイス（Prompt Suggestion）の自動生成。



---

## 📂 プロジェクト構造

* `src/parser.rs`: `pest` を使用した Mumei 構文解析器。
* `src/verification.rs`: Z3を使用した形式検証および `report.json` の出力。
* `src/codegen.rs`: `inkwell` を使用した LLVM IR 生成。
* `visualizer/app.py`: Streamlitベースの検証結果ダッシュボード。
* `grammar.pest`: Mumei の文法定義ファイル。

---

## 🗺️ ロードマップ (Roadmap)

* [x] **Mumei Visualizer:** 検証プロセスの可視化（実装済み）。
* [ ] **Mumei Transpiler:** 検証済みコードを Rust/C++ へ変換。
* [ ] **Self-Healing Loop:** 検証に失敗した際、AIが自動で論理を修正する機能。

---
