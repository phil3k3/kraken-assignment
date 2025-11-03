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
    #[error("Missing transaction type")]
    MissingTransactionType,
    #[error("Missing client")]
    MissingClient,
    #[error("Missing transaction id")]
    MissingTransactionId,
    #[error("Amount missing")]
    MissingAmount,
    #[error("Negative amount")]
    NegativeAmount,
    #[error("Unknown transaction type")]
    UnknownTransactionType,
    #[error("Transaction not found")]
    NoTransaction,
    #[error("Dispute not found")]
    NoDispute,
}
