// ============================================================
// Mumei Verification Suite: Comprehensive Feature Demonstration
// ============================================================
// Covers: Refinement Types, Structs, Generics, Traits, Laws,
//         Loop Invariants, Termination, Float Verification,
//         Stack Safety, Geometric Invariants, Std Library
//
// Standard Library (also available via import):
//   import "std/option" as option;
//   import "std/stack" as stack;
//   import "std/result" as result;
//   import "std/list" as list;

// --- Refinement Types ---
type Nat = i64 where v >= 0;
type Pos = f64 where v > 0.0;

// --- Struct: Geometric Point ---
struct Point {
    x: f64 where v >= 0.0,
    y: f64 where v >= 0.0
}

// --- Generics: Pair<T, U> ---
struct Pair<T, U> {
    first: T,
    second: U
}

// --- Generics: Option<T> (std/option) ---
enum Option<T> {
    Some(T),
    None
}

// --- Trait with Laws ---
trait Comparable {
    fn leq(a: Self, b: Self) -> bool;
    law reflexive: leq(x, x) == true;
}

impl Comparable for i64 {
    fn leq(a: i64, b: i64) -> bool { a <= b }
}

// ============================================================
// Atom 1: Loop Invariant + Termination
// Proves: sum accumulation with invariant s >= 0, i <= n
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
// Atom 2: Float Refinement
// Proves: Pos > 0.0 => result > 0.0
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
// Atom 3: Stack Push Guard (Overflow Prevention)
// Proves: top < max => top+1 <= max
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
// Atom 4: Stack Pop Guard (Underflow Prevention)
// Proves: top > 0 => top-1 >= 0
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
// Atom 5: Circle Area (Geometric Invariant)
// Proves: r > 0 => area > 0
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
// Atom 6: Robust Stack - Bounded Push
// Proves: push onto non-full stack preserves 0 <= top' <= max
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
// Atom 7: Stack Clear with Termination Proof
// Proves: loop terminates (decreases: i) AND invariant preserved
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
// Atom 8: Distance Squared (Non-negative Guarantee)
// Proves: dx^2 + dy^2 >= 0
// ============================================================
atom dist_squared(dx: Nat, dy: Nat)
requires:
    dx >= 0 && dy >= 0;
ensures:
    result >= 0;
body: {
    dx * dx + dy * dy
};
