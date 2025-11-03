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

