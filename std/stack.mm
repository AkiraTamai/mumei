// =============================================================
// Mumei Standard Library: Stack<T>
// =============================================================
// 境界付きスタック。top は現在の要素数、max は最大容量。
// push/pop の安全性を精緻型と事前条件で保証する。
//
// Usage:
//   import "std/stack" as stack;
type Nat = i64 where v >= 0;
struct Stack<T> {
    top: i64 where v >= 0,
    max: i64 where v > 0
}
atom stack_push(top: Nat, max: Nat)
requires:
    top >= 0 && max > 0 && top < max;
ensures:
    result >= 0 && result <= max;
body: {
    top + 1
};
atom stack_pop(top: Nat)
requires:
    top > 0;
ensures:
    result >= 0;
body: {
    top - 1
};
atom stack_is_empty(top: Nat)
requires:
    top >= 0;
ensures:
    result >= 0 && result <= 1;
body: {
    if top == 0 { 1 } else { 0 }
};
atom stack_is_full(top: Nat, max: Nat)
requires:
    top >= 0 && max > 0;
ensures:
    result >= 0 && result <= 1;
body: {
    if top == max { 1 } else { 0 }
};
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
