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
# LLVM: inkwell/llvm-sys 用
export LLVM_SYS_170_PREFIX="$LLVM_PATH"
export PATH="$LLVM_PATH/bin:$PATH"

# Z3: z3-sys 用 (ヘッダーとライブラリの場所を明示)
export Z3_SYS_Z3_HEADER="$Z3_PATH/include/z3.h"
export Z3_SYS_Z3_LIB_DIR="$Z3_PATH/lib"

# コンパイル/リンクフラグ: Cコンパイラ(Clang)が z3.h を見つけるために必要
export CPATH="$Z3_PATH/include:$CPATH"
export LIBRARY_PATH="$Z3_PATH/lib:$LIBRARY_PATH"
export LDFLAGS="-L$LLVM_PATH/lib -L$Z3_PATH/lib"
export CPPFLAGS="-I$LLVM_PATH/include -I$Z3_PATH/include"

echo "✅ Environment configured:"
echo "   - LLVM: $LLVM_PATH (Linking as 17.0)"
echo "   - Z3  : $Z3_PATH"

# --- 3. ビルド工程 ---
echo "🧹 Cleaning previous build artifacts..."
cargo clean

echo "🔨 Building Mumei Compiler..."
if ! cargo build --release; then
    echo "❌ Error: Build failed. Check the errors above."
    exit 1
fi
echo "✨ Build Success!"

# --- 4. テスト用ソースコードの生成 (sword_test.mm) ---
if [ ! -f "sword_test.mm" ]; then
    echo "📝 Creating sword_test.mm..."
    cat <<EOF > sword_test.mm
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
fi

# --- 5. コンパイル実行 ---
echo "🚀 Running Mumei on sword_test.mm..."
# 出力ディレクトリを作成
mkdir -p dist
./target/release/mumei sword_test.mm --output dist/katana

echo "✨ Process complete."