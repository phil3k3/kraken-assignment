use crate::account::{Account, AccountError};
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

pub fn parse_csv(file: &str, buffer_capacity: usize) -> Result<HashMap<u16, Account>> {
    let file = File::open(file)?;
    let buffered_reader = BufReader::with_capacity(buffer_capacity, file);
    let mut reader = ReaderBuilder::new()
        .has_headers(true)                // your sample has a header row
        .flexible(true)
        .trim(csv::Trim::All)// faster when row length is fixed
        .buffer_capacity(buffer_capacity) // if your csv crate version supports it
        .from_reader(buffered_reader);

    let mut accounts: HashMap<u16, Account> = HashMap::new();

    let mut record = ByteRecord::new();
    while reader.read_byte_record(&mut record)? {
        let line_number = reader.position().line();

        let transaction_type = record.get(0)
            .ok_or(Error::MissingTransactionType(line_number))
            .and_then(|raw| parse_transaction_type(raw, line_number))?;
        let client = record.get(1)
            .ok_or(Error::MissingClient(line_number))
            .and_then(|client| lexical_core::parse::<u16>(client).map_err(Error::from))?;
        let transaction_id = record.get(2)
            .ok_or(Error::MissingTransactionId(line_number))
            .and_then(|transaction_id| lexical_core::parse::<u64>(transaction_id).map_err(Error::from))?;

        let amount_row: Option<Amount> = record.get(3)
            .map(|raw| parse_scaled_value(raw, line_number))
            .transpose()?
            .flatten();

        let account = accounts
            .entry(client)
            .or_insert_with_key(|&client| Account::new(client));

        match transaction_type {
            TransactionType::Deposit => {
                let amount = amount_row.ok_or(Error::MissingAmount(line_number))?;
                account.deposit(transaction_id, amount);
            }
            TransactionType::Withdrawal => {
                let amount = amount_row.ok_or(Error::MissingAmount(line_number))?;
                account.withdraw(transaction_id, amount);
            }
            TransactionType::Dispute => {
                account.dispute(transaction_id).map_err(|err| match err {
                    AccountError::NoTransaction(tx_id) => Error::NoTransaction(tx_id, line_number),
                    AccountError::NoDispute(tx_id) => Error::NoDispute(tx_id, line_number),
                })?;
            }
            TransactionType::Resolve => {
                account.resolve(transaction_id).map_err(|err| match err {
                    AccountError::NoTransaction(tx_id) => Error::NoTransaction(tx_id, line_number),
                    AccountError::NoDispute(tx_id) => Error::NoDispute(tx_id, line_number),
                })?;
            }
            TransactionType::Chargeback => {
                account.chargeback(transaction_id).map_err(|err| match err {
                    AccountError::NoTransaction(tx_id) => Error::NoTransaction(tx_id, line_number),
                    AccountError::NoDispute(tx_id) => Error::NoDispute(tx_id, line_number),
                })?;
            }
        }
    }

    Ok(accounts)
}

#[inline]
fn parse_transaction_type(raw: &[u8], line_number: u64) -> Result<TransactionType> {
    // Avoid allocations: compare against byte literals after trimming.
    let b = trim_ascii(raw);
    match b {
        b"deposit"     => Ok(TransactionType::Deposit),
        b"withdrawal"  => Ok(TransactionType::Withdrawal),
        b"dispute"     => Ok(TransactionType::Dispute),
        b"resolve"     => Ok(TransactionType::Resolve),
        b"chargeback"   => Ok(TransactionType::Chargeback),
        _              => Err(Error::UnknownTransactionType(line_number)),
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
fn parse_scaled_value(byte_array: &[u8], line_number: u64) -> Result<Option<Amount>> {
    let byte_array = trim_ascii(byte_array);
    if byte_array.is_empty() { return Ok(None); }
    if byte_array[0] == b'-' { return Err(Error::NegativeAmount(line_number)); }
    let scaled_value: ConstScaleFpdec<i64, 4> = from_utf8(byte_array)?
        .trim()
        .parse()?;
    Ok(Some(scaled_value))
}

// TODO tests for dispute behavior and states

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_csv_basic_transactions() {
        let buffer_capacity = 8192; // Small buffer for testing
        let result = parse_csv("tests/fixtures/test_transactions.csv", buffer_capacity);

        assert!(result.is_ok(), "Failed to process CSV: {:?}", result.err());
        let accounts = result.unwrap();

        // Check that we have 2 clients
        assert_eq!(accounts.len(), 2, "Expected 2 accounts");

        // Check client 1
        let account1 = accounts.get(&1).expect("Client 1 should exist");
        assert_eq!(account1.client, 1);
        // Client 1: deposit 100.0, deposit 50.25, withdrawal 25.0 = 125.25
        // After dispute and resolve of tx 3 (50.25), funds should be back to available
        assert_eq!(account1.funds_available.to_string(), "125.25");
        assert_eq!(account1.funds_held.to_string(), "0");
        assert!(!account1.locked, "Client 1 should not be locked");

        // Check client 2
        let account2 = accounts.get(&2).expect("Client 2 should exist");
        assert_eq!(account2.client, 2);
        // Client 2: deposit 200.5, withdrawal 50.0 = 150.5
        // After dispute and chargeback of tx 2 (200.5), funds_held reduced by 200.5
        // Available: 150.5 - 200.5 (disputed) = -50.0
        // After chargeback, held is reduced to 0, and account is locked
        assert_eq!(account2.funds_held.to_string(), "0");
        assert!(account2.locked, "Client 2 should be locked after chargeback");
    }

    #[test]
    fn test_process_csv_missing_file() {
        let buffer_capacity = 8192;
        let result = parse_csv("nonexistent.csv", buffer_capacity);

        assert!(result.is_err(), "Should fail when file doesn't exist");
    }

    #[test]
    fn test_trim_ascii() {
        assert_eq!(trim_ascii(b"  hello  "), b"hello");
        assert_eq!(trim_ascii(b"hello"), b"hello");
        assert_eq!(trim_ascii(b"  "), b"");
        assert_eq!(trim_ascii(b""), b"");
        assert_eq!(trim_ascii(b" \t\n"), b"");
    }

    #[test]
    fn test_parse_transaction_type() {
        assert!(matches!(parse_transaction_type(b"deposit", 1), Ok(TransactionType::Deposit)));
        assert!(matches!(parse_transaction_type(b"withdrawal", 1), Ok(TransactionType::Withdrawal)));
        assert!(matches!(parse_transaction_type(b"dispute", 1), Ok(TransactionType::Dispute)));
        assert!(matches!(parse_transaction_type(b"resolve", 1), Ok(TransactionType::Resolve)));
        assert!(matches!(parse_transaction_type(b"chargeback", 1), Ok(TransactionType::Chargeback)));
        assert!(matches!(parse_transaction_type(b"invalid", 1), Err(Error::UnknownTransactionType(1))));
        assert!(matches!(parse_transaction_type(b"  deposit  ", 1), Ok(TransactionType::Deposit)));
    }

    #[test]
    fn test_parse_mu_u32_1e4() {
        // Valid amounts
        assert!(parse_scaled_value(b"100.0", 1).unwrap().is_some());
        assert!(parse_scaled_value(b"0.1234", 1).unwrap().is_some());
        assert!(parse_scaled_value(b"  50.25  ", 1).unwrap().is_some());

        // Empty amount
        assert!(parse_scaled_value(b"", 1).unwrap().is_none());
        assert!(parse_scaled_value(b"   ", 1).unwrap().is_none());

        // Negative amount should error
        assert!(matches!(parse_scaled_value(b"-100.0", 1), Err(Error::NegativeAmount(1))));
    }
}
