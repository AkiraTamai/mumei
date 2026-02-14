import os
import subprocess
import json
import tempfile
from pathlib import Path
from mcp.server.fastmcp import FastMCP
from dotenv import load_dotenv

# 環境変数の読み込み
load_dotenv()

# MCPサーバーの初期化
mcp = FastMCP("Mumei-Forge")

@mcp.tool()
def forge_blade(source_code: str, output_name: str = "katana") -> str:
    """
    Mumei言語のコードを一時ディレクトリで検証・コンパイルし、各言語のソースを出力します。
    並行実行しても他のリクエストと干渉しません。
    """
    # プロジェクトのルートディレクトリ（このファイルがある場所）を特定
    root_dir = Path(__file__).parent.absolute()

    # 1. スレッドセーフな一時ディレクトリを作成
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)
        source_path = tmp_path / "input.mm"

        # 思考内容を一時ファイルに書き出し
        source_path.write_text(source_code, encoding="utf-8")

        # 2. コンパイラ実行 (cwdをプロジェクトルートに固定し、出力先を一時フォルダに指定)
        output_base = tmp_path / output_name

        result = subprocess.run(
            ["cargo", "run", "--", str(source_path), "--output", str(output_base)],
            cwd=root_dir,
            capture_output=True,
            text=True
        )

        if result.returncode == 0:
            # 成功時：生成された成果物（.rs, .go, .tsなど）の内容を集約して返す
            response_parts = [f"✅ 鍛造成功: '{output_name}'"]

            # 生成される可能性のある拡張子を走査
            for ext in [".rs", ".go", ".ts", ".ll"]:
                gen_file = tmp_path / f"{output_name}{ext}"
                if gen_file.exists():
                    content = gen_file.read_text(encoding="utf-8")
                    response_parts.append(f"\n### {output_name}{ext}\n```\n{content}\n```")

            return "\n".join(response_parts)
        else:
            # 失敗時：エラー出力（Z3の反例など）を返す
            return f"❌ 鍛造失敗 (論理欠陥検出):\n{result.stderr}"

@mcp.tool()
def inspect_flaws() -> str:
    """
    最新の検証レポートを読み取り、論理の反例（バグの原因）を返します。
    """
    report_path = Path(__file__).parent / "visualizer" / "report.json"
    if not report_path.exists():
        return "検証レポートが見つかりません。先に forge_blade を実行してください。"

    try:
        with open(report_path, "r", encoding="utf-8") as f:
            report = json.load(f)
        return json.dumps(report, indent=2, ensure_ascii=False)
    except Exception as e:
        return f"レポート読み込みエラー: {str(e)}"

@mcp.tool()
def self_heal_loop() -> str:
    """
    self_healing.py を実行し、AIによる自律修正ループ（sword_test.mm対象）を開始します。
    """
    root_dir = Path(__file__).parent.absolute()
    result = subprocess.run(
        ["python", "self_healing.py"],
        cwd=root_dir,
        capture_output=True,
        text=True
    )
    if result.returncode == 0:
        return f"✅ 自律修正完了:\n{result.stdout}"
    else:
        return f"❌ 自律修正失敗:\n{result.stderr}\n{result.stdout}"

if __name__ == "__main__":
    mcp.run()