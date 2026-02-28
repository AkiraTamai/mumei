// =============================================================
// Mumei Standard Library: Result<T, E>
// =============================================================
// 成功 (Ok, tag=0) または失敗 (Err, tag=1) を表すジェネリック型。
// エラーハンドリングの安全性を Z3 で保証する。
//
// Usage:
//   import "std/result" as result;

enum Result<T, E> {
    Ok(T),
    Err(E)
}

// Result が Ok かどうかを判定する
atom is_ok(res: i64)
    requires: res >= 0 && res <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match res {
            0 => 1,
            1 => 0,
            _ => 0
        }
    }

// Result が Err かどうかを判定する
atom is_err(res: i64)
    requires: res >= 0 && res <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match res {
            0 => 0,
            1 => 1,
            _ => 0
        }
    }

// Ok の値を取り出す。Err の場合はデフォルト値を返す。
atom unwrap_or_default(res: i64, default_val: i64)
    requires: res >= 0 && res <= 1;
    ensures: true;
    body: {
        match res {
            0 => default_val,
            _ => default_val
        }
    }

// 安全な除算: ゼロ除算を Err として返す
atom safe_divide(a: i64, b: i64)
    requires: true;
    ensures: result >= 0 && result <= 1;
    body: {
        if b == 0 { 1 } else { 0 }
    }

// =============================================================
// 高階関数相当の操作（Map / AndThen）
// =============================================================

// --- Map 相当: Ok の中身に変換を適用 ---
// res が Ok(tag=0) なら mapped_value を返し、Err(tag=1) なら default_val を返す。
atom result_map_apply(res: i64, default_val: i64, mapped_value: i64)
    requires: res >= 0 && res <= 1;
    ensures: true;
    body: {
        match res {
            0 => mapped_value,
            _ => default_val
        }
    }

// --- AndThen (FlatMap) 相当: Result を返す関数の連鎖 ---
// res が Ok なら inner_res をそのまま返す。Err ならそのまま Err(1) を返す。
atom result_and_then(res: i64, inner_res: i64)
    requires: res >= 0 && res <= 1 && inner_res >= 0 && inner_res <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match res {
            0 => inner_res,
            _ => 1
        }
    }

// --- OrElse: Err の場合に代替 Result を提供 ---
// res が Ok ならそのまま返し、Err なら alternative を返す。
atom result_or_else(res: i64, alternative: i64)
    requires: res >= 0 && res <= 1 && alternative >= 0 && alternative <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match res {
            0 => 0,
            _ => alternative
        }
    }

// --- MapErr: Err の中身を変換 ---
// res が Err なら mapped_err を返し、Ok ならそのまま Ok(0) を返す。
atom result_map_err(res: i64, mapped_err: i64)
    requires: res >= 0 && res <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match res {
            0 => 0,
            _ => mapped_err
        }
    }
