// =============================================================
// 検証済み状態遷移マシン: ATM (Automated Teller Machine)
// =============================================================
// Enum + match + Refinement Types を組み合わせ、
// 「残高不足での引き出し」や「無効な状態遷移」が
// コンパイル時に不可能であることを証明する。
// --- 精緻型: 非負残高 ---
type Balance = i64 where v >= 0;
// --- Enum: ATM の状態 ---
// 0 = Idle, 1 = Authenticated, 2 = Dispensing, 3 = Error
enum AtmState {
    Idle,
    Authenticated,
    Dispensing,
    Error
}
// =============================================================
// ATM 状態遷移: 安全な次状態の計算
// =============================================================
atom atm_transition(state, action, balance: Balance)
    requires: state >= 0 && state <= 3 && action >= 0 && action <= 3;
    ensures: result >= 0 && result <= 3;
    body: {
        match state {
            0 => match action {
                0 => 1,
                _ => 3
            },
            1 => match action {
                1 => 2,
                3 => 0,
                _ => 3
            },
            2 => match action {
                2 if balance > 0 => 0,
                2 => 3,
                3 => 0,
                _ => 3
            },
            _ => 3
        }
    }
// =============================================================
// ATM 残高計算: 引き出し後の残高
// =============================================================
atom atm_withdraw(balance: Balance, amount)
    requires: amount > 0 && amount <= balance;
    ensures: result >= 0;
    body: balance - amount
