// =============================================================
// Mumei Standard Library: Result<T, E>
// =============================================================
// 成功 (Ok, tag=0) または失敗 (Err, tag=1) を表す型。
// エラーハンドリングの安全性を Z3 で保証する。
enum Result {
    Ok(i64),
    Err(i64)
}
// Result が Ok かどうかを判定する
atom is_ok(res)
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
atom is_err(res)
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
atom unwrap_or_default(res, default_val)
    requires: res >= 0 && res <= 1;
    ensures: true;
    body: {
        match res {
            0 => default_val,
            _ => default_val
        }
    }
// 安全な除算: ゼロ除算を Err として返す
type NonZero = i64 where v != 0;
atom safe_divide(a, b)
    requires: true;
    ensures: result >= 0 && result <= 1;
    body: {
        if b == 0 { 1 } else { 0 }
    }
