#!/bin/bash

# --- 1. Homebrew パスの動的取得 ---
LLVM_PATH=$(brew --prefix llvm@18)
Z3_PATH=$(brew --prefix z3)

# 各ツールの存在確認
if [ ! -d "$LLVM_PATH" ]; then
    echo "❌ Error: llvm@18 is not installed. (brew install llvm@18)"
    exit 1
fi
if [ ! -d "$Z3_PATH" ]; then
    echo "❌ Error: z3 is not installed. (brew install z3)"
    exit 1
fi

# --- 2. LLVM & Z3 環境変数の設定 (macOS Apple Silicon 対応) ---
# LLVM 18 を使用するための設定
export LLVM_SYS_180_PREFIX="$LLVM_PATH" # 180 (LLVM 18) に更新
export PATH="$LLVM_PATH/bin:$PATH"

# Z3: z3-sys 用
export Z3_SYS_Z3_HEADER="$Z3_PATH/include/z3.h"
export Z3_SYS_Z3_LIB_DIR="$Z3_PATH/lib"

# コンパイル/リンクフラグ
export CPATH="$Z3_PATH/include:$CPATH"
export LIBRARY_PATH="$Z3_PATH/lib:$LIBRARY_PATH"
export LDFLAGS="-L$LLVM_PATH/lib -L$Z3_PATH/lib"
export CPPFLAGS="-I$LLVM_PATH/include -I$Z3_PATH/include"

echo "✅ Environment configured:"
echo "   - LLVM: $LLVM_PATH (Linking as 18.0)"
echo "   - Z3  : $Z3_PATH"

# --- 3. ビルド工程 ---
echo "🧹 Cleaning previous build artifacts..."
# 頻繁なビルドを考慮し、clean は必要に応じて手動で行う方が速いですが、
# 環境変数を変えた直後は clean するのが安全です。
cargo clean

echo "🔨 Building Mumei Compiler (Refinement Types Support)..."
if ! cargo build --release; then
    echo "❌ Error: Build failed. Check the errors above."
    exit 1
fi
echo "✨ Build Success!"

# --- 4. テスト用ソースコードの生成 (sword_test.mm) ---
# 精緻型 (Refinement Types) を含む最新の構文に更新
echo "📝 Creating/Updating sword_test.mm with Refinement Types..."
cat <<EOF > sword_test.mm
// Define Refinement Type: Natural numbers (non-negative)
type Nat = i64 where v >= 0;

atom sword_sum(n)
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
EOF

# --- 5. コンパイル実行 ---
echo "🚀 Running Mumei on sword_test.mm..."
# 出力ディレクトリを作成
mkdir -p dist

# parser::parse_module を使用する最新の main.rs を実行
if ! ./target/release/mumei sword_test.mm --output dist/katana; then
    echo "❌ Error: Mumei execution failed."
    exit 1
fi

echo "---"
echo "✅ Verification and Code Generation Complete!"
echo "📍 LLVM IR  : dist/katana.ll"
echo "📍 Rust     : dist/katana.rs"
echo "📍 Go       : dist/katana.go"
echo "📍 TS       : dist/katana.ts"
echo "✨ Process complete."