atom bad_consume(ref x: i64)
    consume x;
    requires: true;
    ensures: true;
    body: { x }
