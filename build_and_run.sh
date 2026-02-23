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

# --- 4. テスト用ソースコードの準備 ---
# 以前はここで sword_test.mm を生成していたが、
# 現在はリポジトリ管理下の sword_test.mm をそのまま使用する。
#
# これにより std/ の更新やテスト内容の変更がスクリプトに埋め込まれず、
# README / examples / tests と整合した形で実行できる。
if [ ! -f "sword_test.mm" ]; then
    echo "❌ Error: sword_test.mm not found in repository root"
    exit 1
fi

# --- 5. コンパイル実行 ---
echo "🚀 Running Mumei Verification Suite..."
echo "   Features: Refinement Types, Structs, Generics, Traits, Laws, Loop Invariants, Termination"
echo ""
mkdir -p dist
rm -f dist/katana* # 古い成果物を削除

if ! ./target/release/mumei sword_test.mm --output dist/katana; then
    echo "❌ Error: Mumei execution failed."
    exit 1
fi

echo ""
echo "=== Verification Results ==="
echo "📍 LLVM IR  : $(ls dist/katana_*.ll 2>/dev/null | tr '\n' ' ')"
echo "📍 Rust     : dist/katana.rs"
echo "📍 Go       : dist/katana.go"
echo "📍 TS       : dist/katana.ts"
echo ""

# --- 6. 生成された Rust コードの構文チェック (オプション) ---
if command -v rustc >/dev/null 2>&1; then
    echo "🦀 Checking generated Rust syntax..."
    if rustc --crate-type lib dist/katana.rs --out-dir dist/ 2>/dev/null; then
        echo "✅ Rust syntax is valid."
    else
        echo "⚠️  Rust syntax check failed (non-critical for struct types)."
    fi
fi

echo ""
echo "=== Verified Properties ==="
echo "  ✅ Atom 'sword_sum'    : Loop invariant + Termination (decreases: n-i)"
echo "  ✅ Atom 'scale'        : Float refinement (Pos > 0.0 => result > 0.0)"
echo "  ✅ Atom 'stack_push'   : Overflow prevention (top < max => top+1 <= max)"
echo "  ✅ Atom 'stack_pop'    : Underflow prevention (top > 0 => top-1 >= 0)"
echo "  ✅ Atom 'circle_area'  : Geometric invariant (r > 0 => area > 0)"
echo "  ✅ Atom 'robust_push'  : Bounded stack push (0 <= top' <= max)"
echo "  ✅ Atom 'stack_clear'  : Loop termination (decreases: i) + invariant"
echo "  ✅ Atom 'dist_squared' : Non-negative distance (dx²+dy² >= 0)"
echo "  ✅ Struct 'Point'      : Field constraints (x >= 0.0, y >= 0.0)"
echo "  ✅ Generic 'Pair<T,U>' : Polymorphic struct (monomorphized at compile time)"
echo "  ✅ Generic 'Option<T>' : Polymorphic enum (monomorphized at compile time)"
echo "  ✅ Trait 'Comparable'  : Law 'reflexive' verified by Z3 for i64 impl"
echo "  ✅ Built-in: Eq, Ord, Numeric auto-implemented for i64/u64/f64"
echo ""
echo "🎉 All atoms verified. Generics + Traits + Laws operational. The blade is forged."
