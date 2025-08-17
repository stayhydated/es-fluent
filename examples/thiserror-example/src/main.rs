mod error;
pub mod i18n;

use error::TransactionError;
use es_fluent::ToFluentString as _;

use crate::error::{LockedReason, NetworkError, NotFoundReason};

fn debit_account(
    account: u64,
    amount: u32,
    balance: u32,
    name: &str,
) -> Result<(), TransactionError> {
    if name == "a" {
        return Err(TransactionError::AccountLocked {
            reason: LockedReason::Frozen,
        });
    }
    if name == "b" {
        return Err(TransactionError::AccountLocked {
            reason: LockedReason::Suspended {
                until: "forever".to_string(),
                sensitive_data: "sensitive".to_string(),
            },
        });
    }
    if account == 69 {
        return Err(TransactionError::AccountNotFound(NotFoundReason::NotExist(
            account,
        )));
    }
    if balance < amount {
        return Err(TransactionError::InsufficientFunds {
            available: balance,
            required: amount,
        });
    }
    Ok(())
}

fn main() {
    i18n::init();

    run("en");

    run("fr");

    run("cn");
}

fn run(locale: &str) {
    i18n::change_locale(locale).unwrap();

    println!("Language : {}", locale);

    let tests = [
        debit_account(69, 50, 100, ""),
        debit_account(1, 150, 100, ""),
        debit_account(1, 1, 100, "a"),
        debit_account(1, 1, 100, "b"),
        Err(TransactionError::from(NetworkError::ApiUnavailable)),
    ];

    for res in tests {
        match res {
            Ok(_) => panic!("Unexpected success"),
            Err(e) => {
                println!("thiserror: {}", e);
                println!("i18n: {}", e.to_fluent_string());
                println!();
            }
        }
    }
    println!()
}