use es_fluent::EsFluent;
use thiserror::Error;

#[derive(Debug, EsFluent)]
pub enum LockedReason {
    Frozen,

    Suspended {
        until: String,
        #[fluent(skip)]
        sensitive_data: String,
    },

    Overdue(u32),
}

#[derive(Debug, EsFluent)]
pub enum NotFoundReason {
    NotExist(u64),
}

// here, we use thiserror::Error which uses std::fmt::Display + Debug, and es_fluent::FluentDisplay at the same time.
#[derive(Debug, Error, EsFluent)]
pub enum TransactionError {
    #[error("Account is locked: {reason:?}")]
    AccountLocked { reason: LockedReason },

    #[error("Insufficient funds: available {available}, required {required}")]
    InsufficientFunds { available: u32, required: u32 },

    #[error("Account not found: {0:?}")]
    AccountNotFound(NotFoundReason),

    // skipped since we don't want to generate the underlying fluent keys
    // for that error, since it has its own implementation
    #[fluent(skip)]
    #[error(transparent)]
    Network(#[from] NetworkError),
}

// here, we use thiserror::Error which uses std::fmt::Display + Debug, and es_fluent::FluentDisplay at the same time.
#[derive(Clone, Debug, Error, EsFluent)]
pub enum NetworkError {
    #[error("API is unavailable")]
    ApiUnavailable,
}
