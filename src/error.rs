use std::str::Utf8Error;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Error(#[from] std::io::Error),
    #[error(transparent)]
    Error2(#[from] csv::Error),
    #[error(transparent)]
    Error3(#[from] Utf8Error),
    #[error("Amount missing")]
    MissingAmount,
    #[error("Transaction not found")]
    NoTransaction,
    #[error("Dispute not found")]
    NoDispute,

}
