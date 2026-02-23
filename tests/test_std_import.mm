// std import の統合テスト
import "std/option" as option;
import "std/stack" as stack;
type Nat = i64 where v >= 0;
// std/stack の stack_push を呼び出すテスト
atom test_push(top: Nat, max: Nat)
requires:
    top >= 0 && max > 0 && top < max;
ensures:
    result >= 0 && result <= max;
body: {
    stack_push(top, max)
};
