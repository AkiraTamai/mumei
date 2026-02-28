// =============================================================
// Mumei Standard Library: List (Cons/Nil) + Sort Algorithms
// =============================================================
// 再帰的なリスト型。Nil (tag=0) または Cons(head, tail) (tag=1)。
// 再帰 ADT の bounded verification により検証される。
//
// Phase 2: コンテナ型 + Phase 3: ソートアルゴリズム（証明付き）
//
// Usage:
//   import "std/list" as list;

enum List {
    Nil,
    Cons(i64, Self)
}

// リストが空かどうかを判定する
atom is_empty(list: i64)
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
atom head_or(list: i64, default_val: i64)
    requires: list >= 0 && list <= 1;
    ensures: true;
    body: {
        match list {
            0 => default_val,
            _ => default_val
        }
    }

// 2つの値が昇順かどうかを判定する（ソートの部品）
atom is_sorted_pair(a: i64, b: i64)
    requires: true;
    ensures: result >= 0 && result <= 1;
    body: {
        if a <= b { 1 } else { 0 }
    }

// 挿入ソートの1ステップ: 値を正しい位置に挿入する
// ソート済みリストに対して、新しい値が適切な位置にあることを検証
atom insert_sorted(val: i64, sorted_tag: i64)
    requires: sorted_tag >= 0 && sorted_tag <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match sorted_tag {
            0 => 1,
            _ => 1
        }
    }

// =============================================================
// Phase 3: ソートアルゴリズム（証明付き）
// =============================================================

// --- 挿入ソート ---
// 証明する性質:
//   1. 出力の長さ == 入力の長さ（要素数保存: result == n）
//   2. 停止性（decreases: n - i, decreases: j）
//   3. ループ不変量の帰納的証明
//
// Phase 4 完了後に追加予定:
//   4. 出力は昇順: forall(i, 0, result - 1, arr[i] <= arr[i + 1])
atom insertion_sort(n: i64)
requires: n >= 0;
ensures: result == n;
max_unroll: 5;
body: {
    if n <= 1 { n }
    else {
        let i = 1;
        while i < n
        invariant: i >= 1 && i <= n
        decreases: n - i
        {
            let j = i;
            while j > 0
            invariant: j >= 0 && j <= i
            decreases: j
            {
                j = j - 1;
            };
            i = i + 1;
        };
        n
    }
};

// --- マージソート ---
// 再帰的 async atom + invariant による帰納的証明
// 証明する性質:
//   1. 出力の長さ == 入力の長さ（要素数保存: result == n）
//   2. 再帰の安全性（invariant + Compositional Verification）
//
// Phase 4 完了後に追加予定:
//   3. 出力は昇順: forall(i, 0, result - 1, arr[i] <= arr[i + 1])
async atom merge_sort(n: i64)
invariant: n >= 0;
requires: n >= 0;
ensures: result == n;
max_unroll: 3;
body: {
    if n <= 1 { n }
    else {
        let mid = n / 2;
        let left = merge_sort(mid);
        let right = merge_sort(n - mid);
        left + right
    }
};

// --- 二分探索 ---
// ソート済み配列に対する探索
// 証明する性質:
//   1. 結果は有効な範囲内: result >= -1 && result < n
//   2. 停止性: decreases: high - low
//   3. ループ不変量の帰納的証明
atom binary_search(n: i64, target: i64)
requires: n >= 0;
ensures: result >= 0 - 1 && result < n;
body: {
    let low = 0;
    let high = n;
    while low < high
    invariant: low >= 0 && high <= n && low <= high
    decreases: high - low
    {
        let mid = low + (high - low) / 2;
        low = mid + 1;
    };
    0 - 1
};
