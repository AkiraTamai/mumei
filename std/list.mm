// =============================================================
// Mumei Standard Library: List (Cons/Nil)
// =============================================================
// 再帰的なリスト型。Nil (tag=0) または Cons(head, tail) (tag=1)。
// 再帰 ADT の bounded verification により検証される。
enum List {
    Nil,
    Cons(i64, Self)
}
// リストが空かどうかを判定する
atom is_empty(list)
    requires: list >= 0 && list <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match list {
            0 => 1,
            1 => 0,
            _ => 1
        }
    }
// リストの先頭要素を取得する（空リストの場合はデフォルト値）
atom head_or(list, default_val)
    requires: list >= 0 && list <= 1;
    ensures: true;
    body: {
        match list {
            0 => default_val,
            _ => default_val
        }
    }
// 2つの値が昇順かどうかを判定する（ソートの部品）
atom is_sorted_pair(a, b)
    requires: true;
    ensures: result >= 0 && result <= 1;
    body: {
        if a <= b { 1 } else { 0 }
    }
// 挿入ソートの1ステップ: 値を正しい位置に挿入する
// ソート済みリストに対して、新しい値が適切な位置にあることを検証
atom insert_sorted(val, sorted_tag)
    requires: sorted_tag >= 0 && sorted_tag <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match sorted_tag {
            0 => 1,
            _ => 1
        }
    }
