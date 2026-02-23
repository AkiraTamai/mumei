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
