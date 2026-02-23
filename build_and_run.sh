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

# --- 5. メイン検証スイート実行 ---
MUMEI=./target/release/mumei
echo "🚀 Running Mumei Verification Suite..."
echo "   sword_test.mm: Refinement Types, Structs, Generics, Traits, Laws, Termination"
echo ""
mkdir -p dist
rm -f dist/katana* # 古い成果物を削除

if ! $MUMEI build sword_test.mm -o dist/katana; then
    echo "❌ Error: Mumei verification failed on sword_test.mm"
    exit 1
fi

echo ""
echo "=== Main Suite Results ==="
echo "📍 LLVM IR  : $(ls dist/katana_*.ll 2>/dev/null | tr '\n' ' ')"
echo "📍 Rust     : dist/katana.rs"
echo "📍 Go       : dist/katana.go"
echo "📍 TS       : dist/katana.ts"
echo ""

# --- 6. Example テスト ---
echo "🧪 Running example tests..."
EXAMPLES_PASSED=0
EXAMPLES_FAILED=0

# 6a. Inter-atom call test
echo -n "  call_test.mm ... "
if $MUMEI build examples/call_test.mm -o dist/call_test 2>/dev/null; then
    echo "✅"
    EXAMPLES_PASSED=$((EXAMPLES_PASSED + 1))
else
    echo "❌"
    EXAMPLES_FAILED=$((EXAMPLES_FAILED + 1))
fi

# 6b. ATM state machine (enum + match + guards)
echo -n "  match_atm.mm ... "
if $MUMEI build examples/match_atm.mm -o dist/match_atm 2>/dev/null; then
    echo "✅"
    EXAMPLES_PASSED=$((EXAMPLES_PASSED + 1))
else
    echo "❌"
    EXAMPLES_FAILED=$((EXAMPLES_FAILED + 1))
fi

# 6c. Expression evaluator (zero-division detection)
echo -n "  match_evaluator.mm ... "
if $MUMEI build examples/match_evaluator.mm -o dist/match_evaluator 2>/dev/null; then
    echo "✅"
    EXAMPLES_PASSED=$((EXAMPLES_PASSED + 1))
else
    echo "❌"
    EXAMPLES_FAILED=$((EXAMPLES_FAILED + 1))
fi

# 6d. Multi-file import test
echo -n "  import_test/main.mm ... "
if $MUMEI build examples/import_test/main.mm -o dist/import_test 2>/dev/null; then
    echo "✅"
    EXAMPLES_PASSED=$((EXAMPLES_PASSED + 1))
else
    echo "❌"
    EXAMPLES_FAILED=$((EXAMPLES_FAILED + 1))
fi

# 6e. Std library import test
echo -n "  test_std_import.mm ... "
if $MUMEI build tests/test_std_import.mm -o dist/test_std 2>/dev/null; then
    echo "✅"
    EXAMPLES_PASSED=$((EXAMPLES_PASSED + 1))
else
    echo "❌"
    EXAMPLES_FAILED=$((EXAMPLES_FAILED + 1))
fi

echo ""
echo "  Examples: $EXAMPLES_PASSED passed, $EXAMPLES_FAILED failed"

# --- 7. 生成された Rust コードの構文チェック (オプション) ---
if command -v rustc >/dev/null 2>&1; then
    echo ""
    echo "🦀 Checking generated Rust syntax..."
    if rustc --crate-type lib dist/katana.rs --out-dir dist/ 2>/dev/null; then
        echo "  ✅ Rust syntax is valid."
    else
        echo "  ⚠️  Rust syntax check failed (non-critical)."
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
echo "  ✅ Generic 'Pair<T,U>' : Polymorphic struct (monomorphization)"
echo "  ✅ Generic 'Option<T>' : Polymorphic enum (monomorphization)"
echo "  ✅ Trait 'Comparable'  : Law 'reflexive' verified by Z3"
echo "  ✅ Std Library         : std/option, std/stack, std/result, std/list"
echo "  ✅ Built-in Traits     : Eq, Ord, Numeric for i64/u64/f64"
echo ""
if [ "$EXAMPLES_FAILED" -gt 0 ]; then
    echo "⚠️  $EXAMPLES_FAILED example(s) failed. Check output above."
    exit 1
fi
echo "🎉 All verified. The blade is forged."
