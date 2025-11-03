use std::str::Utf8Error;
use primitive_fixed_point_decimal::ParseError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // System errors
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    LexicalParse(#[from] lexical_core::Error),

    // User errors
    #[error("Missing transaction type on line {0}")]
    MissingTransactionType(u64),
    #[error("Missing client on line {0}")]
    MissingClient(u64),
    #[error("Missing transaction id on line {0}")]
    MissingTransactionId(u64),
    #[error("Amount missing on line {0}")]
    MissingAmount(u64),
    #[error("Negative amount on line {0}")]
    NegativeAmount(u64),
    #[error("Unknown transaction type on line {0}")]
    UnknownTransactionType(u64),
    #[error("Transaction id {0} not found for dispute on line {1}")]
    NoTransaction(u64, u64),
    #[error("Dispute not found for resolve/chargeback of transaction id {0} on line {1}")]
    NoDispute(u64, u64),
}
