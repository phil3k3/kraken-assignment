use crate::account::Account;
use crate::error::Error;
use bigdecimal::BigDecimal;
use csv::WriterBuilder;
use std::collections::HashMap;
use std::fs::File;
use crate::prelude::*;

#[derive(Debug, serde::Deserialize)]
enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "chargeback")]
    Chargeback
}

#[derive(Debug, serde::Deserialize)]
struct Transaction {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    #[serde(rename = "client")]
    client: u16,
    #[serde(rename = "tx")]
    transaction_id: u64,
    #[serde(rename = "amount")]
    amount: Option<BigDecimal>
}

#[derive(Debug, serde::Serialize)]
pub struct AccountRecord {
    client: u16,
    available: BigDecimal,
    held: BigDecimal,
    total: BigDecimal,
    locked: bool
}

impl From<&Account> for AccountRecord {
    fn from(value: &Account) -> Self {
        AccountRecord {
            client: value.client,
            available: value.funds_available.clone(),
            held: value.funds_held.clone(),
            total: &value.funds_held + &value.funds_available,
            locked: value.locked,
        }
    }
}

pub fn write_accounts(accounts: HashMap<u16, Account>) -> Result<String> {
    let mut wtr = WriterBuilder::new().from_writer(vec![]);
    accounts.iter().for_each(|(client, account)| {
        wtr.serialize(AccountRecord::from(account)).unwrap()
    });
    let vec = wtr.into_inner().map_err(|x| Error::from(x.into_error()))?;
    String::from_utf8(vec).map_err(|x| x.utf8_error().into())
}

pub fn process_csv(file: &str) -> Result<HashMap<u16, Account>> {
    let file = File::open(file)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(file);

    let mut accounts: HashMap<u16, Account> = HashMap::new();

    for result in rdr.deserialize::<Transaction>() {
        let row: Transaction = result?;
        let account = accounts.entry(row.client).or_insert_with(|| Account::new(row.client));

        match row.transaction_type {
            TransactionType::Deposit => {
                let amount = row.amount.ok_or(Error::MissingAmount)?;
                account.deposit(row.transaction_id, &amount)?;
            }
            TransactionType::Withdrawal => {
                let amount = row.amount.ok_or(Error::MissingAmount)?;
                account.withdraw(row.transaction_id, &amount)?;
            }
            TransactionType::Dispute => {
                account.dispute(row.transaction_id)?;
            }
            TransactionType::Resolve => {
                account.resolve(row.transaction_id)?;
            }
            TransactionType::Chargeback => {
                account.chargeback(row.transaction_id)?;
            }
        }
    }
    Ok(accounts)
}
