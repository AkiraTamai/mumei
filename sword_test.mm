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
