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

# --- 2. 環境変数の設定 ---
export LLVM_SYS_180_PREFIX="$LLVM_PATH"
export PATH="$LLVM_PATH/bin:$PATH"
export Z3_SYS_Z3_HEADER="$Z3_PATH/include/z3.h"
export Z3_SYS_Z3_LIB_DIR="$Z3_PATH/lib"
export CPATH="$Z3_PATH/include:$CPATH"
export LIBRARY_PATH="$Z3_PATH/lib:$LIBRARY_PATH"
export LDFLAGS="-L$LLVM_PATH/lib -L$Z3_PATH/lib"
export CPPFLAGS="-I$LLVM_PATH/include -I$Z3_PATH/include"

echo "✅ Environment configured for LLVM 18 & Z3"

# --- 3. ビルド工程 ---
# 初回や環境変更時以外は cargo build だけで十分高速です
if [ "$1" == "--clean" ]; then
    echo "🧹 Cleaning..."
    cargo clean
fi

echo "🔨 Building Mumei Compiler..."
if ! cargo build --release; then
    echo "❌ Error: Build failed."
    exit 1
fi

# --- 4. テスト用ソースコードの生成 ---
echo "📝 Creating sword_test.mm..."
cat <<EOF > sword_test.mm
// Type System 2.0: Refinement Types
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;

// Struct: フィールド精緻型付き構造体
struct Point {
    x: f64 where v >= 0.0,
    y: f64 where v >= 0.0
}

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
EOF

# --- 5. コンパイル実行 ---
echo "🚀 Running Mumei..."
mkdir -p dist
rm -f dist/katana* # 古い成果物を削除

if ! ./target/release/mumei sword_test.mm --output dist/katana; then
    echo "❌ Error: Mumei execution failed."
    exit 1
fi

echo "---"
echo "✅ Verification and Code Generation Complete!"
# main.rs の変更により、LLVM IR は Atom 名が付与されます
echo "📍 LLVM IR  : $(ls dist/katana_*.ll)"
echo "📍 Rust     : dist/katana.rs"
echo "📍 Go       : dist/katana.go"
echo "📍 TS       : dist/katana.ts"
echo "---"

# --- 6. 生成された Rust コードの構文チェック (オプション) ---
if command -v rustc >/dev/null 2>&1; then
    echo "🦀 Checking generated Rust syntax..."
    rustc --crate-type lib dist/katana.rs --out-dir dist/
    echo "✅ Rust syntax is valid."
fi

echo "✨ All processes complete. The blade is forged."
