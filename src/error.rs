use std::str::Utf8Error;
use primitive_fixed_point_decimal::ParseError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Utf8(#[from] Utf8Error),
    #[error("Amount missing")]
    MissingAmount,
    #[error("Transaction not found")]
    NoTransaction,
    #[error("Dispute not found")]
    NoDispute,
    #[error("Unknown transaction type")]
    UnknownTransactionType,
    #[error("Negative amount")]
    NegativeAmount,
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("Missing client")]
    MissingClient,
    #[error(transparent)]
    LexicalParse(#[from] lexical_core::Error),
    #[error("Missing transaction type")]
    MissingTransactionType,
    #[error("Missing transaction id")]
    MissingTransactionId,
}
