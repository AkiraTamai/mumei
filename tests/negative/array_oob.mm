atom unsafe_access(arr: i64, n: i64)
    requires: n >= 0;
    ensures: true;
    body: { arr[n] }
