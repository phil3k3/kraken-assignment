use crate::account::Account;
use crate::error::Error;
use crate::prelude::*;
use csv::{ByteRecord, ReaderBuilder, WriterBuilder};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

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

impl From<&Account> for AccountRecord {
    fn from(value: &Account) -> Self {
        AccountRecord {
            client: value.client,
            available: format_mu_1e4(value.funds_available),
            held: format_mu_1e4(value.funds_held),
            total: format_mu_1e4(value.funds_held + value.funds_available),
            locked: value.locked
        }
    }
}


#[inline]
fn parse_mu_u32_1e4(b: &[u8]) -> Option<u32> {
    // Accepts "12", "12.3", "12.34", "12.3456". Rejects >4 dp or negatives.
    let b = trim_ascii(b);
    if b.is_empty() { return None; }
    if b[0] == b'-' { return None; } // no negative amounts here

    let mut i = 0usize;
    let n = b.len();

    let mut int_part: u64 = 0;
    while i < n && b[i].is_ascii_digit() {
        int_part = int_part.checked_mul(10)?.checked_add((b[i] - b'0') as u64)?;
        i += 1;
    }

    let mut frac: u32 = 0;
    let mut dp = 0u8;
    if i < n && b[i] == b'.' {
        i += 1;
        while i < n && b[i].is_ascii_digit() && dp < 5 { // read an extra to detect >4
            if dp < 4 { frac = frac * 10 + (b[i] - b'0') as u32; }
            dp += 1;
            i += 1;
        }
    }
    if i != n || dp > 4 { return None; }

    // Scale frac to 4 decimal places (e.g., "1.5" -> frac=5, dp=1 -> frac=5000)
    if dp > 0 && dp < 4 {
        frac *= 10u32.pow(4 - dp as u32);
    }

    // int_part * 10000 + frac must fit in u32
    let total = int_part.checked_mul(10_000)?.checked_add(frac as u64)?;
    u32::try_from(total).ok()
}
#[inline]
fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && bytes[start].is_ascii_whitespace() { start += 1; }
    while end > start && bytes[end - 1].is_ascii_whitespace() { end -= 1; }
    &bytes[start..end]
}


pub fn write_accounts(accounts: HashMap<u16, Account>) -> Result<String> {
    let mut wtr = WriterBuilder::new().from_writer(vec![]);
    accounts
        .iter()
        .for_each(|(_client, account)| wtr.serialize(AccountRecord::from(account)).unwrap());
    let vec = wtr.into_inner().map_err(|x| Error::from(x.into_error()))?;
    String::from_utf8(vec).map_err(|x| x.utf8_error().into())
}

pub fn process_csv(file: &str) -> Result<HashMap<u16, Account>> {
    let file = File::open(file)?;
    // TODO think about capacity
    let reader = BufReader::with_capacity(32 * 1024 * 1024, file);
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)                // your sample has a header row
        .flexible(true)
        .trim(csv::Trim::All)// faster when row length is fixed
        .buffer_capacity(32 * 1024 * 1024) // if your csv crate version supports it
        .from_reader(reader);

    let mut accounts: HashMap<u16, Account> = HashMap::new();

    let mut rec = ByteRecord::new();
    while rdr.read_byte_record(&mut rec)? {
        // TODO unwrap
        let transaction_type = rec.get(0).map(|x| {
            parse_transaction_type(x).unwrap()
        }).unwrap();
        let client = lexical_core::parse::<u16>(rec.get(1).unwrap()).unwrap();
        let transaction_id = lexical_core::parse::<u64>(rec.get(2).unwrap()).unwrap();

        let amount_row : Option<u32> = rec.get(3).map(|x| {
            parse_mu_u32_1e4(x)
        }).flatten();

        let account = accounts
            .entry(client)
            .or_insert_with(|| Account::new(client));

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

fn format_mu_1e4(value: i64) -> String {
    let int_part = value / 10_000;
    let frac_part = value % 10_000;
    format!("{}.{:04}", int_part, frac_part)
}

// TODO tests for conversion
// TODO tests for dispute behavior and states
