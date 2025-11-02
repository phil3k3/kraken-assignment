use crate::error::Error;
use std::collections::HashMap;

#[derive(Default)]
pub struct Account {
    pub client: u16,
    pub funds_available: i64,
    pub funds_held: i64,
    disputes: HashMap<u64, u32>,
    disputable_transactions: HashMap<u64, u32>,
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
        amount: u32,
    ) -> crate::prelude::Result<()> {
        self.funds_available -= amount as i64;
        self.disputable_transactions
            .insert(transaction_id, amount);
        Ok(())
    }

    pub(crate) fn deposit(
        &mut self,
        transaction_id: u64,
        amount: u32,
    ) -> crate::prelude::Result<()> {
        self.funds_available += amount as i64;
        self.disputable_transactions
            .insert(transaction_id, amount);
        Ok(())
    }

    pub(crate) fn resolve(&mut self, transaction_id: u64) -> crate::prelude::Result<()> {
        let disputed_amount = self
            .disputes
            .remove(&transaction_id)
            .ok_or(Error::NoDispute)?;
        self.funds_available += disputed_amount as i64;
        self.funds_held -= disputed_amount as i64;
        self.disputable_transactions
            .insert(transaction_id, disputed_amount);
        Ok(())
    }

    pub(crate) fn chargeback(&mut self, transaction_id: u64) -> crate::prelude::Result<()> {
        let disputed_amount = self
            .disputes
            .remove(&transaction_id)
            .ok_or(Error::NoDispute)?;
        self.funds_held -= disputed_amount as i64;
        self.locked = true;
        // assume no more disputes possible on that account
        Ok(())
    }

    pub(crate) fn dispute(&mut self, transaction_id: u64) -> crate::prelude::Result<()> {
        let disputed_amount = self
            .disputable_transactions
            .remove(&transaction_id)
            .ok_or(Error::NoTransaction)?;
        self.funds_available -= disputed_amount as i64;
        self.funds_held += disputed_amount as i64;
        self.disputes.insert(transaction_id, disputed_amount);
        Ok(())
    }
}

