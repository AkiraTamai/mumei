# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language for AI Agents.**

「無銘（Mumei）」は、作者の個性を排し、数学的な「正しさ」のみを追求するAIネイティブなプログラミング言語です。AIがコードを生成する際、実行前にその論理的欠陥を数学的に証明・排除し、不純物のない「真実のコード」のみをマシンコード（LLVM）および検証済みソースコード（Rust/Go/TS）へと昇華させます。

---

## 🛠️ 設計思想 (Design Philosophy)

Mumeiは以下の5つの工程（鍛造プロセス）を経て、実行バイナリ、検証済みソース、および検証レポートを生成します。

1. **Polishing (Parser):** `atom` と呼ばれる極小の関数単位でコードを解析します。
2. **The Ritual of Truth (Verification):** Z3 SMT Solverを用い、事前条件 (`requires`) が実装 (`body`) の安全性を数学的に担保しているか検証します。
3. **Visual Inspection (Visualizer):** 検証で発見された「論理の亀裂（反例）」をリアルタイムで視覚化します。
4. **Tempering (Codegen):** 検証をパスしたコードを LLVM IR へと変換し、高速な実行能力を与えます。
5. **Sharpening (Transpiler):** 検証済みロジックを、ドキュメントとアサーション付きの高品質な **Rust/Go/TypeScript** コードとして出力します。

---

## 🚀 セットアップ (Installation)

### 1. 依存ライブラリの導入

* **LLVM 15:** ネイティブコード生成用
* **Z3 Solver:** 論理検証用
* **Python 3.x:** ビジュアライザー、修復スクリプト、MCPサーバー用

```bash
# macOS
brew install llvm@15 z3

# Ubuntu
sudo apt install llvm-15-dev libz3-dev

# Python dependencies
pip install streamlit pandas python-dotenv openai mcp-server-fastmcp

```

### 2. 環境変数の設定

ルートディレクトリに `.env` ファイルを作成してください。 **※ `.env` ファイルは Git の追跡から除外（.gitignoreに追加）してください。**

```text
OPENAI_API_KEY=your_api_key_here

```

---

## 🤖 MCP Server (AI Agent Integration)

Mumeiは **Model Context Protocol (MCP)** に対応しています。
最新のサーバー実装では、リクエストごとに一時ディレクトリを作成して隔離（サンドボックス化）するため、並行実行時もデータの競合が発生しません。

### 1. Claude Desktop への登録

`claude_desktop_config.json` に設定を追記します。

```json
{
  "mcpServers": {
    "mumei": {
      "command": "python",
      "args": ["/絶対パス/to/mumei/mcp_server.py"],
      "env": {
        "OPENAI_API_KEY": "your_api_key_here"
      }
    }
  }
}

```

### 2. 提供されるツール (Tools)

* **`forge_blade`**: Mumeiコードを検証・コンパイル・マルチ言語変換し、**検証レポートを含めて**一括で返却します（並行安全）。
* **`self_heal_loop`**: ローカルの `sword_test.mm` を対象に、検証をパスするまでAIが自律的に修正を行います。

---

## 📖 使い方 (Usage)

### 1. 手動での鍛造

```bash
# 検証結果(report.json)は指定した出力先のディレクトリに生成されます
cargo run -- sword_test.mm --output katana

```

### 2. 自律修復 (Self-Healing Loop)

検証に失敗した場合、AIが自動的にエラーログと反例を分析し、ソースコードを修正します。

```bash
python self_healing.py

```

---

## 📊 Inspection (Visualizer)

Mumeiは単に「エラー」を返すだけでなく、**なぜその論理が破綻したのか**を具体的な数値で提示します。

* **起動:** `streamlit run visualizer/app.py`
* **機能:** 検証失敗時の **反例 (Counter-example)** の提示、およびAI用修正プロンプトの自動生成。

---

## 📂 プロジェクト構造

* `src/main.rs`: 鍛造プロセスの司令塔。出力先ディレクトリに基づくレポートパス管理を実装。
* `src/verification.rs`: Z3を使用した形式検証。隔離されたパスへの `report.json` 出力に対応。
* `src/transpiler/`: Rust, Go, TypeScript への変換ロジック。
* `self_healing.py`: OpenAI APIを利用した自律的論理修正スクリプト。
* `mcp_server.py`: 並行安全性を確保した一時ディレクトリ方式の MCP サーバー。

---

## 🗺️ ロードマップ (Roadmap)

* [x] **Mumei Visualizer:** 検証プロセスの可視化。
* [x] **Mumei Transpiler (Rust):** 検証済みコードの Rust 変換。
* [x] **Self-Healing Loop:** AIによる自律的な論理修正。
* [x] **Stateless MCP Server:** 並行安全な一時ディレクトリ方式の実装。
* [ ] **Multi-Language Support:** Go および TypeScript トランスパイラの完備。

---
