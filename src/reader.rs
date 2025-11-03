use crate::account::Account;
use crate::error::Error;
use crate::prelude::*;
use csv::{ByteRecord, ReaderBuilder, WriterBuilder};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::str::from_utf8;
use primitive_fixed_point_decimal::ConstScaleFpdec;
use crate::Amount;

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
    Chargeback,
}

#[derive(Debug, serde::Serialize)]
pub struct AccountRecord {
    client: u16,
    available: String,
    held: String,
    total: String,
    locked: bool,
}

impl From<Account> for AccountRecord {
    fn from(account: Account) -> Self {
        AccountRecord {
            client: account.client,
            available: account.funds_available.to_string(),
            held: account.funds_held.to_string(),
            total: (account.funds_held + account.funds_available).to_string(),
            locked: account.locked
        }
    }
}


pub fn write_accounts(accounts: HashMap<u16, Account>) -> Result<String> {
    let mut writer = WriterBuilder::new().from_writer(vec![]);
    for (_client_id, account) in accounts {
        writer.serialize(AccountRecord::from(account))?;
    }
    let vec = writer.into_inner().map_err(|err| Error::from(err.into_error()))?;
    String::from_utf8(vec).map_err(|err| err.utf8_error().into())
}

pub fn process_csv(file: &str) -> Result<HashMap<u16, Account>> {
    let file = File::open(file)?;
    //
    let buffered_reader = BufReader::with_capacity(32 * 1024 * 1024, file);
    let mut reader = ReaderBuilder::new()
        .has_headers(true)                // your sample has a header row
        .flexible(true)
        .trim(csv::Trim::All)// faster when row length is fixed
        .buffer_capacity(32 * 1024 * 1024) // if your csv crate version supports it
        .from_reader(buffered_reader);

    let mut accounts: HashMap<u16, Account> = HashMap::new();

    let mut record = ByteRecord::new();
    while reader.read_byte_record(&mut record)? {
        let transaction_type = record.get(0)
            .ok_or(Error::MissingTransactionType)
            .and_then(parse_transaction_type)?;
        let client = record.get(1).ok_or(Error::MissingClient)
            .and_then(|client| lexical_core::parse::<u16>(client).map_err(Error::from))?;
        let transaction_id = record.get(2).ok_or(Error::MissingTransactionId)
            .and_then(|transaction_id| lexical_core::parse::<u64>(transaction_id).map_err(Error::from))?;

        let amount_row: Option<Amount> = record.get(3)
            .map(parse_mu_u32_1e4)
            .transpose()?
            .flatten();

        let account = accounts
            .entry(client)
            .or_insert_with_key(|&client| Account::new(client));

        match transaction_type {
            TransactionType::Deposit => {
                let amount = amount_row.ok_or(Error::MissingAmount)?;
                account.deposit(transaction_id, amount)?;
            }
            TransactionType::Withdrawal => {
                let amount = amount_row.ok_or(Error::MissingAmount)?;
                account.withdraw(transaction_id, amount)?;
            }
            TransactionType::Dispute => {
                account.dispute(transaction_id)?;
            }
            TransactionType::Resolve => {
                account.resolve(transaction_id)?;
            }
            TransactionType::Chargeback => {
                account.chargeback(transaction_id)?;
            }
        }
    }

    Ok(accounts)
}

#[inline]
fn parse_transaction_type(raw: &[u8]) -> Result<TransactionType> {
    // Avoid allocations: compare against byte literals after trimming.
    let b = trim_ascii(raw);
    match b {
        b"deposit"     => Ok(TransactionType::Deposit),
        b"withdrawal"  => Ok(TransactionType::Withdrawal),
        b"dispute"     => Ok(TransactionType::Dispute),
        b"resolve"     => Ok(TransactionType::Resolve),
        b"chargeback"   => Ok(TransactionType::Chargeback),
        _              => Err(Error::UnknownTransactionType),
    }
}

#[inline]
fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && bytes[start].is_ascii_whitespace() { start += 1; }
    while end > start && bytes[end - 1].is_ascii_whitespace() { end -= 1; }
    &bytes[start..end]
}



#[inline]
fn parse_mu_u32_1e4(b: &[u8]) -> Result<Option<Amount>> {
    let b = trim_ascii(b);
    if b.is_empty() { return Ok(None); }
    if b[0] == b'-' { return Err(Error::NegativeAmount); }
    let s = from_utf8(b)?.trim();
    let v: ConstScaleFpdec<i64, 4> = s.parse()?;
    Ok(Some(v))
}


// TODO tests for conversion
// TODO tests for dispute behavior and states
