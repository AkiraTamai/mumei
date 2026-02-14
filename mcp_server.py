import os
import subprocess
import json
from mcp.server.fastmcp import FastMCP
from dotenv import load_dotenv

# 環境変数の読み込み
load_dotenv()

# MCPサーバーの初期化
mcp = FastMCP("Mumei-Forge")

@mcp.tool()
def forge_blade(source_code: str, output_name: str = "katana") -> str:
    """
    Mumei言語のコードをコンパイル、検証、Rust変換します。
    """
    # 1. ソースファイルを書き出し
    source_path = "sword_test.mm"
    with open(source_path, "w") as f:
        f.write(source_code)

    # 2. コンパイラ実行
    result = subprocess.run(
        ["cargo", "run", "--", source_path, "--output", output_name],
        capture_output=True, text=True
    )

    if result.returncode == 0:
        return f"✅ 鍛造成功: {output_name}.ll および {output_name}.rs が生成されました。\n{result.stdout}"
    else:
        return f"❌ 鍛造失敗 (論理欠陥検出):\n{result.stderr}"

@mcp.tool()
def inspect_flaws() -> str:
    """
    最新の検証レポートを読み取り、論理の反例（バグの原因）を返します。
    """
    report_path = "visualizer/report.json"
    if not os.path.exists(report_path):
        return "レポートが見つかりません。"

    with open(report_path, "r") as f:
        report = json.load(f)
    return json.dumps(report, indent=2, ensure_ascii=False)

@mcp.tool()
def self_heal_loop() -> str:
    """
    self_healing.py を実行し、AIによる自律修正ループをトリガーします。
    """
    result = subprocess.run(
        ["python", "self_healing.py"],
        capture_output=True, text=True
    )
    return result.stdout

if __name__ == "__main__":
    mcp.run()