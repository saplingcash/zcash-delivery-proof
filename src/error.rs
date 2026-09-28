use std::fmt;

/// Why making or checking a proof failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not a transaction librustzcash reads back to the same bytes.
    Transaction(String),
    /// The proof string or bytes are malformed.
    Proof(String),
    /// The viewing key is not a unified full or incoming viewing key with an Orchard item.
    Key(String),
    /// The proof does not hold for this transaction: the reason says which part failed.
    Mismatch(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Transaction(m) | Error::Proof(m) | Error::Key(m) | Error::Mismatch(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}
