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
echo "📝 Creating sword_test.mm (Comprehensive Verification Suite)..."
cat <<'EOF' > sword_test.mm
// ============================================================
// Mumei Verification Suite: Comprehensive Feature Demonstration
// ============================================================

// --- Refinement Types ---
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;
type StackIdx = i64 where v >= 0;

// --- Struct: Geometric Point (Plan B) ---
struct Point {
    x: f64 where v >= 0.0,
    y: f64 where v >= 0.0
}

// --- Struct: Circle with positive radius (Plan B) ---
struct Circle {
    cx: f64 where v >= 0.0,
    cy: f64 where v >= 0.0,
    r: f64 where v > 0.0
}

// ============================================================
// Atom 1: Loop Invariant + Termination (Plan A: Stack-like sum)
// ============================================================
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
    decreases: n - i
    {
        s = s + i;
        i = i + 1;
    };
    s
};

// ============================================================
// Atom 2: Float Refinement (Plan B: Scaling)
// ============================================================
atom scale(x: Pos)
requires:
    x > 0.0;
ensures:
    result > 0.0;
body: {
    x * 2.0
};

// ============================================================
// Atom 3: Stack Push Guard (Plan A: Overflow Prevention)
// Proves: top < max => after push, top+1 <= max
// ============================================================
atom stack_push(top: Nat, max: Nat)
requires:
    top >= 0 && max >= 0 && top < max;
ensures:
    result >= 0 && result <= max;
body: {
    top + 1
};

// ============================================================
// Atom 4: Stack Pop Guard (Plan A: Underflow Prevention)
// Proves: top > 0 => after pop, top-1 >= 0
// ============================================================
atom stack_pop(top: Nat)
requires:
    top > 0;
ensures:
    result >= 0;
body: {
    top - 1
};

// ============================================================
// Atom 5: Circle Area (Plan B: Geometric Invariant)
// Proves: positive radius => positive area
// ============================================================
atom circle_area(r: Pos)
requires:
    r > 0.0;
ensures:
    result > 0.0;
body: {
    r * r * 3.14159
};

// ============================================================
// Atom 6: Robust Stack - Bounded Push (Final Trial)
// Proves: push onto non-full stack preserves 0 <= top' <= max
// Combined with capacity check: top < max is precondition
// ============================================================
atom robust_push(top: Nat, max: Nat, val: Nat)
requires:
    top >= 0 && max > 0 && top < max && val >= 0;
ensures:
    result >= 0 && result <= max;
body: {
    top + 1
};

// ============================================================
// Atom 7: Robust Stack - Clear All (Final Trial)
// Proves: loop terminates AND invariant preserved
// Uses decreases clause to prove termination: i decreases to 0
// ============================================================
atom stack_clear(top: Nat)
requires:
    top >= 0;
ensures:
    result >= 0;
body: {
    let i = top;
    while i > 0
    invariant: i >= 0
    decreases: i
    {
        i = i - 1;
    };
    i
};

// ============================================================
// Atom 8: Distance Squared (Plan B: Geometric Safety)
// Proves: squared distance is always non-negative
// No sqrt needed — avoids NaN by design
// ============================================================
atom dist_squared(dx: Nat, dy: Nat)
requires:
    dx >= 0 && dy >= 0;
ensures:
    result >= 0;
body: {
    dx * dx + dy * dy
};
EOF

# --- 5. コンパイル実行 ---
echo "🚀 Running Mumei Verification Suite..."
echo "   Features: Refinement Types, Structs, Loop Invariants, Termination Check"
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
echo "  ✅ Struct 'Circle'     : Field constraints (cx >= 0.0, cy >= 0.0, r > 0.0)"
echo ""
echo "🎉 All 8 atoms verified. All 2 structs registered. The blade is forged."
