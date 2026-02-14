# 🗡️ Mumei (無銘)

**Mathematical Proof-Driven Programming Language for AI Agents.**

「無銘（Mumei）」は、作者の個性を排し、数学的な「正しさ」のみを追求するAIネイティブなプログラミング言語です。AIがコードを生成する際、実行前にその論理的欠陥を数学的に証明・排除し、不純物のない「真実のコード」のみをマシンコード（LLVM）および検証済みソースコード（Rust）へと昇華させます。

---

## 🛠️ 設計思想 (Design Philosophy)

Mumeiは以下の5つの工程（鍛造プロセス）を経て、実行バイナリ、検証済みソース、および検証レポートを生成します。

1. **Polishing (Parser):** `atom` と呼ばれる極小の関数単位でコードを解析します。
2. **The Ritual of Truth (Verification):** Z3 SMT Solverを用い、事前条件 (`requires`) が実装 (`body`) の安全性を数学的に担保しているか検証します。
3. **Visual Inspection (Visualizer):** 検証で発見された「論理の亀裂（反例）」をリアルタイムで視覚化します。
4. **Tempering (Codegen):** 検証をパスしたコードを LLVM IR へと変換し、高速な実行能力を与えます。
5. **Sharpening (Transpiler):** 検証済みロジックを、ドキュメントとアサーション付きの高品質な **Rust** コードとして出力します。

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

ルートディレクトリに `.env` ファイルを作成してください。 **※ `.env` ファイルは Git の追跡から除外してください。**

```text
OPENAI_API_KEY=your_api_key_here

```

---

## 🤖 MCP Server (AI Agent Integration)

Mumeiは **Model Context Protocol (MCP)** に対応しています。これを導入することで、Claude などの AI エージェントが自ら「Mumei 職人」としてコードの鍛造と修正を行うようになります。

### 1. Claude Desktop への登録

`claude_desktop_config.json` に以下の設定を追記します。

* **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
* **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

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

* **`forge_blade`**: Mumeiコードを書き出し、検証・コンパイル・Rust変換を一括実行。
* **`inspect_flaws`**: 検証失敗時の反例（レポート）を取得。
* **`self_heal_loop`**: 検証をパスするまでAIが自律的にコードを修正。

---

## 📖 使い方 (Usage)

### 1. 手動での鍛造

```bash
# 検証、LLVM IR生成、Rustコード変換を一度に行います
cargo run -- sword_test.mm --output katana

```

### 2. 自律修復 (Self-Healing Loop)

検証に失敗した場合、AIが自動的にエラーログと反例を分析し、`.mm` ファイルを修正します。

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

* `src/main.rs`: 鍛造プロセスのオーケストレーション（終了コード管理）。
* `src/verification.rs`: Z3を使用した形式検証および `report.json` の出力。
* `src/transpiler.rs`: 検証済み `atom` を Rust ソースコードへ変換。
* `src/codegen.rs`: LLVM IR 生成。
* `self_healing.py`: AIによる自律的な論理修正スクリプト。
* `mcp_server.py`: AIエージェント接続用 MCP サーバー。
* `visualizer/app.py`: Streamlitベースの検証結果ダッシュボード。

---

## 🗺️ ロードマップ (Roadmap)

* [x] **Mumei Visualizer:** 検証プロセスの可視化。
* [x] **Mumei Transpiler:** 検証済みコードを Rust へ変換。
* [x] **Self-Healing Loop:** AIによる自律的な論理修正機能。
* [x] **Mumei MCP Server:** AIエージェント用インターフェースの実装。

---
