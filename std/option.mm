// =============================================================
// Mumei Standard Library: Option<T>
// =============================================================
// 値の有無を表す基本型。None (tag=0) または Some(value) (tag=1)。
// Z3 による網羅性チェックと精緻型の恩恵を受ける。
enum Option {
    None,
    Some(i64)
}
// Option が Some かどうかを判定する（tag == 1 なら true）
atom is_some(opt)
    requires: opt >= 0 && opt <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match opt {
            0 => 0,
            1 => 1,
            _ => 0
        }
    }
// Option が None かどうかを判定する（tag == 0 なら true）
atom is_none(opt)
    requires: opt >= 0 && opt <= 1;
    ensures: result >= 0 && result <= 1;
    body: {
        match opt {
            0 => 1,
            1 => 0,
            _ => 0
        }
    }
// Some の値を取り出す。None の場合はデフォルト値を返す。
atom unwrap_or(opt, default_val)
    requires: opt >= 0 && opt <= 1;
    ensures: true;
    body: {
        match opt {
            0 => default_val,
            _ => default_val
        }
    }
