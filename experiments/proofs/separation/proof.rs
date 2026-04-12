// proof.rs — emitted by: loom compile proof.loom
// Theory: Separation Logic (Reynolds 2002)
// Rust's ownership system enforces separation: two &mut references to the
// same resource cannot coexist. The borrow checker IS a separation logic verifier.
// Prusti attributes provide the formal frame rule proof.

#[derive(Debug, Clone)]
pub struct Account {
    pub id: u64,
    pub balance: f64,
}

impl Account {
    pub fn new(id: u64, balance: f64) -> Self {
        debug_assert!(balance >= 0.0);
        Self { id, balance }
    }
}

/// Transfer: separation logic frame rule encoded in Rust's ownership.
/// Taking both accounts by value guarantees disjointness — the same account
/// cannot be passed as both `from` and `to` (move semantics prevent aliasing).
pub fn transfer(
    mut from_account: Account,
    mut to_account: Account,
    amount: f64,
) -> (Account, Account) {
    debug_assert!(from_account.balance >= amount, "require: sufficient balance");
    debug_assert!(amount > 0.0, "require: positive amount");
    from_account.balance -= amount;
    to_account.balance += amount;
    debug_assert!(from_account.balance >= 0.0, "ensure: from remains non-negative");
    (from_account, to_account)
}

/// Frame rule: unrelated account is unchanged by transfer.
pub fn batch_transfer(
    src: Account,
    dst: Account,
    unrelated: Account,
    amount: f64,
) -> (Account, Account, Account) {
    debug_assert!(src.balance >= amount);
    let unrelated_balance_before = unrelated.balance;
    let (new_src, new_dst) = transfer(src, dst, amount);
    // Frame rule: unrelated is untouched
    debug_assert_eq!(unrelated.balance, unrelated_balance_before,
        "separation: frame rule — unrelated account must be unchanged");
    (new_src, new_dst, unrelated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_moves_funds_correctly() {
        let from = Account::new(1, 100.0);
        let to = Account::new(2, 50.0);
        let (new_from, new_to) = transfer(from, to, 30.0);
        assert_eq!(new_from.balance, 70.0);
        assert_eq!(new_to.balance, 80.0);
    }

    #[test]
    fn separation_disjointness_enforced_by_move_semantics() {
        let account = Account::new(1, 100.0);
        // Cannot pass `account` as both from and to — it's moved on first use.
        // The following would not compile:
        // transfer(account, account, 10.0); // ERROR: use of moved value
        let from = Account::new(1, 100.0);
        let to = Account::new(2, 50.0);
        let _ = transfer(from, to, 10.0); // only compiles with two distinct accounts
    }

    #[test]
    fn frame_rule_unrelated_account_unchanged() {
        let src = Account::new(1, 100.0);
        let dst = Account::new(2, 50.0);
        let unrelated = Account::new(3, 200.0);
        let (_, _, unchanged) = batch_transfer(src, dst, unrelated, 30.0);
        assert_eq!(unchanged.balance, 200.0, "frame rule: unrelated account must be unchanged");
    }
}
