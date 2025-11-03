use std::collections::HashMap;
use primitive_fixed_point_decimal::ConstScaleFpdec;
use crate::Amount;

#[derive(thiserror::Error, Debug)]
pub enum AccountError {
    #[error("Transaction id {0} not found for dispute")]
    NoTransaction(u64),
    #[error("Dispute not found for resolve/chargeback of transaction id {0}")]
    NoDispute(u64),
}

pub type AccountResult<T> = Result<T, AccountError>;

#[derive(Default)]
pub struct Account {
    pub client: u16,
    pub funds_available: ConstScaleFpdec<i64, 4>,
    pub funds_held: ConstScaleFpdec<i64, 4>,
    disputes: HashMap<u64, Amount>,
    disputable_transactions: HashMap<u64, Amount>,
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
        amount: Amount,
    ) {
        self.funds_available -= amount;
        self.disputable_transactions
            .insert(transaction_id, amount);
    }

    pub(crate) fn deposit(
        &mut self,
        transaction_id: u64,
        amount: Amount,
    ) {
        self.funds_available += amount;
        self.disputable_transactions
            .insert(transaction_id, amount);
    }

    pub(crate) fn resolve(&mut self, transaction_id: u64) -> AccountResult<()> {
        let disputed_amount = self
            .disputes
            .remove(&transaction_id)
            .ok_or(AccountError::NoDispute(transaction_id))?;
        self.funds_available += disputed_amount;
        self.funds_held -= disputed_amount;
        self.disputable_transactions
            .insert(transaction_id, disputed_amount);
        Ok(())
    }

    pub(crate) fn chargeback(&mut self, transaction_id: u64) -> AccountResult<()> {
        let disputed_amount = self
            .disputes
            .remove(&transaction_id)
            .ok_or(AccountError::NoDispute(transaction_id))?;
        self.funds_held -= disputed_amount;
        self.locked = true;
        // assume no more disputes possible on that account
        Ok(())
    }

    pub(crate) fn dispute(&mut self, transaction_id: u64) -> AccountResult<()> {
        let disputed_amount = self
            .disputable_transactions
            .remove(&transaction_id)
            .ok_or(AccountError::NoTransaction(transaction_id))?;
        self.funds_available -= disputed_amount;
        self.funds_held += disputed_amount;
        self.disputes.insert(transaction_id, disputed_amount);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_amount(value: &str) -> Amount {
        value.parse().expect("Failed to parse amount")
    }

    #[test]
    fn test_new_account() {
        let account = Account::new(42);

        assert_eq!(account.client, 42);
        assert_eq!(account.funds_available.to_string(), "0");
        assert_eq!(account.funds_held.to_string(), "0");
        assert!(!account.locked);
    }

    #[test]
    fn test_deposit() {
        let mut account = Account::new(1);
        let amount = create_amount("100.50");

        account.deposit(1, amount);

        assert_eq!(account.funds_available.to_string(), "100.5");
        assert_eq!(account.funds_held.to_string(), "0");
    }

    #[test]
    fn test_multiple_deposits() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        account.deposit(2, create_amount("50.25"));
        account.deposit(3, create_amount("25.75"));

        assert_eq!(account.funds_available.to_string(), "176");
        assert_eq!(account.funds_held.to_string(), "0");
    }

    #[test]
    fn test_withdrawal() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        account.withdraw(2, create_amount("30.0"));

        assert_eq!(account.funds_available.to_string(), "70");
        assert_eq!(account.funds_held.to_string(), "0");
    }

    #[test]
    fn test_withdrawal_can_go_negative() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("50.0"));
        account.withdraw(2, create_amount("75.0"));

        // No check for sufficient funds, so balance can go negative
        assert_eq!(account.funds_available.to_string(), "-25");
    }

    #[test]
    fn test_dispute_moves_funds_to_held() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        let result = account.dispute(1);

        assert!(result.is_ok());
        assert_eq!(account.funds_available.to_string(), "0");
        assert_eq!(account.funds_held.to_string(), "100");
        assert!(!account.locked);
    }

    #[test]
    fn test_dispute_nonexistent_transaction() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        let result = account.dispute(999);

        assert!(matches!(result, Err(AccountError::NoTransaction(999))));
        // Funds should remain unchanged
        assert_eq!(account.funds_available.to_string(), "100");
        assert_eq!(account.funds_held.to_string(), "0");
    }

    #[test]
    fn test_dispute_withdrawal() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        account.withdraw(2, create_amount("30.0"));
        let result = account.dispute(2);

        assert!(result.is_ok());
        // Disputing a withdrawal: available 70 - 30 = 40, held = 30
        assert_eq!(account.funds_available.to_string(), "40");
        assert_eq!(account.funds_held.to_string(), "30");
    }

    #[test]
    fn test_resolve_returns_funds_to_available() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        account.dispute(1).expect("Dispute should succeed");
        let result = account.resolve(1);

        assert!(result.is_ok());
        assert_eq!(account.funds_available.to_string(), "100");
        assert_eq!(account.funds_held.to_string(), "0");
        assert!(!account.locked);
    }

    #[test]
    fn test_resolve_nonexistent_dispute() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        let result = account.resolve(1);

        assert!(matches!(result, Err(AccountError::NoDispute(1))));
        assert_eq!(account.funds_available.to_string(), "100");
        assert_eq!(account.funds_held.to_string(), "0");
    }

    #[test]
    fn test_resolve_makes_transaction_disputable_again() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        account.dispute(1).expect("First dispute should succeed");
        account.resolve(1).expect("Resolve should succeed");

        // After resolve, transaction should be disputable again
        let result = account.dispute(1);
        assert!(result.is_ok());
        assert_eq!(account.funds_available.to_string(), "0");
        assert_eq!(account.funds_held.to_string(), "100");
    }

    #[test]
    fn test_chargeback_locks_account() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        account.dispute(1).expect("Dispute should succeed");
        let result = account.chargeback(1);

        assert!(result.is_ok());
        assert_eq!(account.funds_available.to_string(), "0");
        assert_eq!(account.funds_held.to_string(), "0");
        assert!(account.locked);
    }

    #[test]
    fn test_chargeback_nonexistent_dispute() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        let result = account.chargeback(1);

        assert!(matches!(result, Err(AccountError::NoDispute(1))));
        assert!(!account.locked);
    }

    #[test]
    fn test_chargeback_removes_held_funds() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("200.0"));
        account.deposit(2, create_amount("100.0"));
        account.dispute(1).expect("Dispute should succeed");

        // Before chargeback: available = 100, held = 200
        assert_eq!(account.funds_available.to_string(), "100");
        assert_eq!(account.funds_held.to_string(), "200");

        account.chargeback(1).expect("Chargeback should succeed");

        // After chargeback: available = 100, held = 0 (200 was charged back)
        assert_eq!(account.funds_available.to_string(), "100");
        assert_eq!(account.funds_held.to_string(), "0");
        assert!(account.locked);
    }

    #[test]
    fn test_complex_dispute_scenario() {
        let mut account = Account::new(1);

        // Multiple deposits
        account.deposit(1, create_amount("100.0"));
        account.deposit(2, create_amount("50.0"));
        account.deposit(3, create_amount("25.0"));

        // Withdrawal
        account.withdraw(4, create_amount("30.0"));

        // Total: 100 + 50 + 25 - 30 = 145
        assert_eq!(account.funds_available.to_string(), "145");

        // Dispute deposit of 50
        account.dispute(2).expect("Dispute should succeed");
        assert_eq!(account.funds_available.to_string(), "95");
        assert_eq!(account.funds_held.to_string(), "50");

        // Resolve the dispute
        account.resolve(2).expect("Resolve should succeed");
        assert_eq!(account.funds_available.to_string(), "145");
        assert_eq!(account.funds_held.to_string(), "0");

        // Dispute withdrawal of 30
        account.dispute(4).expect("Dispute withdrawal should succeed");
        assert_eq!(account.funds_available.to_string(), "115");
        assert_eq!(account.funds_held.to_string(), "30");

        // Chargeback the withdrawal dispute
        account.chargeback(4).expect("Chargeback should succeed");
        assert_eq!(account.funds_available.to_string(), "115");
        assert_eq!(account.funds_held.to_string(), "0");
        assert!(account.locked);
    }

    #[test]
    fn test_cannot_dispute_same_transaction_twice() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        account.dispute(1).expect("First dispute should succeed");

        // Second dispute should fail because transaction is no longer disputable
        let result = account.dispute(1);
        assert!(matches!(result, Err(AccountError::NoTransaction(1))));
    }

    #[test]
    fn test_multiple_disputes_on_different_transactions() {
        let mut account = Account::new(1);

        account.deposit(1, create_amount("100.0"));
        account.deposit(2, create_amount("50.0"));
        account.deposit(3, create_amount("75.0"));

        // Dispute all three
        account.dispute(1).expect("Dispute 1 should succeed");
        account.dispute(2).expect("Dispute 2 should succeed");
        account.dispute(3).expect("Dispute 3 should succeed");

        assert_eq!(account.funds_available.to_string(), "0");
        assert_eq!(account.funds_held.to_string(), "225");

        // Resolve one
        account.resolve(2).expect("Resolve should succeed");
        assert_eq!(account.funds_available.to_string(), "50");
        assert_eq!(account.funds_held.to_string(), "175");

        // Chargeback another
        account.chargeback(1).expect("Chargeback should succeed");
        assert_eq!(account.funds_available.to_string(), "50");
        assert_eq!(account.funds_held.to_string(), "75");
        assert!(account.locked);
    }
}
