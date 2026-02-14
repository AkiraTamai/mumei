atom divide_ritual(a, b) {
    requires: b != 0;
    ensures: result * b == a;
    body: a / b;
}