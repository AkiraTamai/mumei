atom no_termination(n: i64)
    requires: n >= 0;
    ensures: result >= 0;
    body: {
        let i = 0;
        while i < n
        invariant: i >= 0
        decreases: i
        {
            i = i + 1;
        };
        i
    }
