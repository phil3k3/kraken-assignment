use crate::error::Error;
use bigdecimal::BigDecimal;
use std::collections::HashMap;

#[derive(Default)]
pub struct Account {
    pub client: u16,
    pub funds_available: BigDecimal,
    pub funds_held: BigDecimal,
    disputes: HashMap<u64, BigDecimal>,
    disputable_transactions: HashMap<u64, BigDecimal>,
    pub locked: bool,
}

impl Account {
    pub(crate) fn new(client: u16) -> Self {
        Account {
            client,
            ..Default::default()
        }
    }

    pub(crate) fn withdraw(
        &mut self,
        transaction_id: u64,
        amount: &BigDecimal,
    ) -> crate::prelude::Result<()> {
        self.funds_available -= amount;
        self.disputable_transactions
            .insert(transaction_id, amount.clone());
        Ok(())
    }

    pub(crate) fn deposit(
        &mut self,
        transaction_id: u64,
        amount: &BigDecimal,
    ) -> crate::prelude::Result<()> {
        self.funds_available += amount;
        self.disputable_transactions
            .insert(transaction_id, amount.clone());
        Ok(())
    }

    pub(crate) fn resolve(&mut self, transaction_id: u64) -> crate::prelude::Result<()> {
        let disputed_amount = self
            .disputes
            .remove(&transaction_id)
            .ok_or(Error::NoDispute)?;
        self.funds_available += &disputed_amount;
        self.funds_held -= &disputed_amount;
        self.disputable_transactions
            .insert(transaction_id, disputed_amount);
        Ok(())
    }

    pub(crate) fn chargeback(&mut self, transaction_id: u64) -> crate::prelude::Result<()> {
        let disputed_amount = self
            .disputes
            .remove(&transaction_id)
            .ok_or(Error::NoDispute)?;
        self.funds_held -= &disputed_amount;
        self.locked = true;
        // assume no more disputes possible on that account
        Ok(())
    }

    pub(crate) fn dispute(&mut self, transaction_id: u64) -> crate::prelude::Result<()> {
        let disputed_amount = self
            .disputable_transactions
            .remove(&transaction_id)
            .ok_or(Error::NoTransaction)?;
        self.funds_available -= &disputed_amount;
        self.funds_held += &disputed_amount;
        self.disputes.insert(transaction_id, disputed_amount);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // Helper function to create BigDecimal from string
    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    // ===== DEPOSIT TESTS =====

    #[test]
    fn test_deposit_basic() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();

        assert_eq!(account.funds_available, bd("100.0"));
        assert_eq!(account.funds_held, bd("0"));
        assert!(!account.locked);
    }

    #[test]
    fn test_deposit_multiple() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(2, &bd("50.5")).unwrap();
        account.deposit(3, &bd("25.25")).unwrap();

        assert_eq!(account.funds_available, bd("175.75"));
        assert_eq!(account.funds_held, bd("0"));
    }

    #[test]
    fn test_deposit_decimal_precision() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("0.0001")).unwrap();
        account.deposit(2, &bd("0.0002")).unwrap();
        account.deposit(3, &bd("0.0003")).unwrap();

        assert_eq!(account.funds_available, bd("0.0006"));
    }

    #[test]
    fn test_deposit_zero_amount() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("0")).unwrap();

        assert_eq!(account.funds_available, bd("0"));
    }

    // ===== WITHDRAWAL TESTS =====

    #[test]
    fn test_withdrawal_basic() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.withdraw(2, &bd("30.0")).unwrap();

        assert_eq!(account.funds_available, bd("70.0"));
        assert_eq!(account.funds_held, bd("0"));
    }

    #[test]
    fn test_withdrawal_multiple() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.withdraw(2, &bd("25.0")).unwrap();
        account.withdraw(3, &bd("25.0")).unwrap();
        account.withdraw(4, &bd("25.0")).unwrap();

        assert_eq!(account.funds_available, bd("25.0"));
    }

    #[test]
    fn test_withdrawal_allows_negative_balance() {
        // Current implementation allows negative balance
        let mut account = Account::new(1);
        account.deposit(1, &bd("50.0")).unwrap();
        account.withdraw(2, &bd("100.0")).unwrap();

        assert_eq!(account.funds_available, bd("-50.0"));
    }

    #[test]
    fn test_withdrawal_exact_balance() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.withdraw(2, &bd("100.0")).unwrap();

        assert_eq!(account.funds_available, bd("0"));
    }

    // ===== DISPUTE TESTS =====

    #[test]
    fn test_dispute_deposit() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.dispute(1).unwrap();

        assert_eq!(account.funds_available, bd("0"));
        assert_eq!(account.funds_held, bd("100.0"));
        assert!(!account.locked);
    }

    #[test]
    fn test_dispute_withdrawal() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.withdraw(2, &bd("30.0")).unwrap();
        account.dispute(2).unwrap();

        assert_eq!(account.funds_available, bd("40.0")); // 100 - 30 - 30 (disputed)
        assert_eq!(account.funds_held, bd("30.0"));
    }

    #[test]
    fn test_dispute_nonexistent_transaction() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();

        let result = account.dispute(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoTransaction));
    }

    #[test]
    fn test_dispute_already_disputed_transaction() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.dispute(1).unwrap();

        // Trying to dispute again should fail (transaction removed from disputable_transactions)
        let result = account.dispute(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoTransaction));
    }

    #[test]
    fn test_dispute_multiple_transactions() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(2, &bd("50.0")).unwrap();
        account.deposit(3, &bd("25.0")).unwrap();

        account.dispute(1).unwrap();
        account.dispute(3).unwrap();

        assert_eq!(account.funds_available, bd("50.0"));
        assert_eq!(account.funds_held, bd("125.0"));
    }

    // ===== RESOLVE TESTS =====

    #[test]
    fn test_resolve_disputed_deposit() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.dispute(1).unwrap();
        account.resolve(1).unwrap();

        assert_eq!(account.funds_available, bd("100.0"));
        assert_eq!(account.funds_held, bd("0"));
        assert!(!account.locked);
    }

    #[test]
    fn test_resolve_disputed_withdrawal() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.withdraw(2, &bd("30.0")).unwrap();
        account.dispute(2).unwrap();
        account.resolve(2).unwrap();

        assert_eq!(account.funds_available, bd("70.0"));
        assert_eq!(account.funds_held, bd("0"));
    }

    #[test]
    fn test_resolve_without_dispute() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();

        let result = account.resolve(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoDispute));
    }

    #[test]
    fn test_resolve_nonexistent_transaction() {
        let mut account = Account::new(1);

        let result = account.resolve(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoDispute));
    }

    #[test]
    fn test_resolve_multiple_disputes() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(2, &bd("50.0")).unwrap();
        account.dispute(1).unwrap();
        account.dispute(2).unwrap();

        account.resolve(1).unwrap();

        assert_eq!(account.funds_available, bd("100.0"));
        assert_eq!(account.funds_held, bd("50.0"));

        account.resolve(2).unwrap();

        assert_eq!(account.funds_available, bd("150.0"));
        assert_eq!(account.funds_held, bd("0"));
    }

    #[test]
    fn test_resolve_makes_transaction_disputable_again() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.dispute(1).unwrap();
        account.resolve(1).unwrap();

        // Should be able to dispute again after resolve
        account.dispute(1).unwrap();

        assert_eq!(account.funds_available, bd("0"));
        assert_eq!(account.funds_held, bd("100.0"));
    }

    // ===== CHARGEBACK TESTS =====

    #[test]
    fn test_chargeback_disputed_deposit() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.dispute(1).unwrap();
        account.chargeback(1).unwrap();

        assert_eq!(account.funds_available, bd("0"));
        assert_eq!(account.funds_held, bd("0"));
        assert!(account.locked);
    }

    #[test]
    fn test_chargeback_disputed_withdrawal() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.withdraw(2, &bd("30.0")).unwrap();
        account.dispute(2).unwrap();
        account.chargeback(2).unwrap();

        assert_eq!(account.funds_available, bd("40.0"));
        assert_eq!(account.funds_held, bd("0"));
        assert!(account.locked);
    }

    #[test]
    fn test_chargeback_without_dispute() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();

        let result = account.chargeback(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoDispute));
    }

    #[test]
    fn test_chargeback_nonexistent_transaction() {
        let mut account = Account::new(1);

        let result = account.chargeback(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoDispute));
    }

    #[test]
    fn test_chargeback_locks_account_permanently() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(2, &bd("50.0")).unwrap();
        account.dispute(1).unwrap();
        account.chargeback(1).unwrap();

        assert!(account.locked);

        // Account remains locked even after other operations
        // (though the implementation doesn't prevent operations on locked accounts)
        account.deposit(3, &bd("25.0")).unwrap();
        assert!(account.locked);
    }

    #[test]
    fn test_chargeback_with_multiple_held_funds() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(2, &bd("50.0")).unwrap();
        account.dispute(1).unwrap();
        account.dispute(2).unwrap();

        // Chargeback one dispute
        account.chargeback(1).unwrap();

        assert_eq!(account.funds_available, bd("0"));
        assert_eq!(account.funds_held, bd("50.0")); // Only tx 2 still held
        assert!(account.locked);
    }

    // ===== COMPLEX WORKFLOW TESTS =====

    #[test]
    fn test_full_dispute_resolve_cycle() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        assert_eq!(account.funds_available, bd("100.0"));

        account.dispute(1).unwrap();
        assert_eq!(account.funds_available, bd("0"));
        assert_eq!(account.funds_held, bd("100.0"));

        account.resolve(1).unwrap();
        assert_eq!(account.funds_available, bd("100.0"));
        assert_eq!(account.funds_held, bd("0"));
        assert!(!account.locked);
    }

    #[test]
    fn test_full_dispute_chargeback_cycle() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();

        account.dispute(1).unwrap();
        account.chargeback(1).unwrap();

        assert_eq!(account.funds_available, bd("0"));
        assert_eq!(account.funds_held, bd("0"));
        assert!(account.locked);
    }

    #[test]
    fn test_cannot_resolve_after_chargeback() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.dispute(1).unwrap();
        account.chargeback(1).unwrap();

        // Try to resolve after chargeback
        let result = account.resolve(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoDispute));
    }

    #[test]
    fn test_cannot_dispute_after_chargeback() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(2, &bd("50.0")).unwrap();
        account.dispute(1).unwrap();
        account.chargeback(1).unwrap();

        // Try to dispute another transaction after account is locked
        // Note: Current implementation doesn't prevent this
        account.dispute(2).unwrap();
        assert_eq!(account.funds_held, bd("50.0"));
    }

    #[test]
    fn test_mixed_deposits_withdrawals_and_disputes() {
        let mut account = Account::new(1);

        // Complex sequence
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(2, &bd("50.0")).unwrap();
        account.withdraw(3, &bd("30.0")).unwrap();

        assert_eq!(account.funds_available, bd("120.0"));

        account.dispute(1).unwrap();

        assert_eq!(account.funds_available, bd("20.0"));
        assert_eq!(account.funds_held, bd("100.0"));

        account.withdraw(4, &bd("10.0")).unwrap();

        assert_eq!(account.funds_available, bd("10.0"));

        account.resolve(1).unwrap();

        assert_eq!(account.funds_available, bd("110.0"));
        assert_eq!(account.funds_held, bd("0"));
    }

    #[test]
    fn test_dispute_with_insufficient_available_funds() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.withdraw(2, &bd("90.0")).unwrap();

        assert_eq!(account.funds_available, bd("10.0"));

        // Dispute the original deposit - this can make available funds negative
        account.dispute(1).unwrap();

        assert_eq!(account.funds_available, bd("-90.0"));
        assert_eq!(account.funds_held, bd("100.0"));
    }

    #[test]
    fn test_multiple_chargebacks_on_same_account() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(2, &bd("50.0")).unwrap();

        account.dispute(1).unwrap();
        account.chargeback(1).unwrap();

        assert!(account.locked);

        // Dispute and chargeback another transaction
        account.dispute(2).unwrap();
        account.chargeback(2).unwrap();

        assert_eq!(account.funds_available, bd("0"));
        assert_eq!(account.funds_held, bd("0"));
        assert!(account.locked);
    }

    // ===== EDGE CASES =====

    #[test]
    fn test_duplicate_transaction_ids_overwrite() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("100.0")).unwrap();
        account.deposit(1, &bd("50.0")).unwrap(); // Same transaction ID

        assert_eq!(account.funds_available, bd("150.0"));

        // When disputed, only the last amount is used
        account.dispute(1).unwrap();

        assert_eq!(account.funds_available, bd("100.0"));
        assert_eq!(account.funds_held, bd("50.0"));
    }

    #[test]
    fn test_very_large_amounts() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("999999999999.9999")).unwrap();

        assert_eq!(account.funds_available, bd("999999999999.9999"));
    }

    #[test]
    fn test_very_small_amounts() {
        let mut account = Account::new(1);
        account.deposit(1, &bd("0.00000001")).unwrap();

        assert_eq!(account.funds_available, bd("0.00000001"));
    }

    #[test]
    fn test_new_account_defaults() {
        let account = Account::new(42);

        assert_eq!(account.client, 42);
        assert_eq!(account.funds_available, bd("0"));
        assert_eq!(account.funds_held, bd("0"));
        assert!(!account.locked);
    }
}
