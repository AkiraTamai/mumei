atom safe_add(n)
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
