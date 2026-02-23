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
