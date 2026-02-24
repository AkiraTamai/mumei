atom partial_match(x: i64)
    requires: true;
    ensures: result >= 0;
    body: {
        match x {
            0 => 1,
            1 => 2
        }
    }
