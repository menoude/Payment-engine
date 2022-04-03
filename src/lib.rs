use {
    serde::Deserialize,
    std::{convert::TryFrom, io::Read},
    strum_macros::{Display, EnumString},
};

pub mod clients;
pub mod transactions;
use {clients::*, transactions::*};

pub use {clients::ClientAccounts, transactions::MoneyOperationsRegister};

#[derive(Debug, Deserialize)]
pub struct TransactionLine {
    #[serde(rename = "type")]
    transaction_type: TransactionKind,
    #[serde(rename = "client")]
    client_id: ClientId,
    #[serde(rename = "tx")]
    transaction_id: TransactionId,
    amount: Option<f64>,
}

#[derive(Debug, Deserialize, Display, EnumString)]
#[serde(rename_all = "lowercase")]
pub enum TransactionKind {
    Deposit,
    Withdrawal,
    Resolve,
    Dispute,
    Chargeback,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    DeserializationError(csv::Error),
    TransactionError(transactions::TransactionError),
    WrongArgument,
}

impl From<csv::Error> for Error {
    fn from(err: csv::Error) -> Self {
        Self::DeserializationError(err)
    }
}

impl From<TransactionError> for Error {
    fn from(err: TransactionError) -> Self {
        Self::TransactionError(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::WrongArgument => "Wrong argument".to_string(),
                Error::DeserializationError(de) => format!("{}", de),
                Error::TransactionError(te) => format!("{}", te),
            }
        )
    }
}

impl std::convert::TryFrom<TransactionLine> for TransactionOrder {
    type Error = Error;

    fn try_from(line: TransactionLine) -> Result<TransactionOrder> {
        Ok(match line.transaction_type {
            TransactionKind::Deposit | TransactionKind::Withdrawal => {
                TransactionOrder::MoneyOperation(MoneyOperation {
                    client_id: line.client_id,
                    transaction_id: line.transaction_id,
                    disputed: false,
                    operation_kind: match (line.transaction_type, line.amount) {
                        (TransactionKind::Deposit, Some(amount)) if amount >= 0. => {
                            OperationKind::Deposit(amount)
                        }
                        (TransactionKind::Withdrawal, Some(amount)) if amount >= 0. => {
                            OperationKind::Withdrawal(amount)
                        }
                        _ => return Err(Error::WrongArgument),
                    },
                })
            }
            _ => TransactionOrder::ClientClaim(ClientClaim {
                transaction_id: line.transaction_id,
                client_id: line.client_id,
                claim_kind: match line.transaction_type {
                    TransactionKind::Resolve => ClientClaimKind::Resolve,
                    TransactionKind::Dispute => ClientClaimKind::Dispute,
                    TransactionKind::Chargeback => ClientClaimKind::Chargeback,
                    _ => panic!("This can't happen"),
                },
            }),
        })
    }
}

pub fn read_transactions_file<R: Read>(
    file: R,
    accounts: &mut ClientAccounts,
    operations_register: &mut MoneyOperationsRegister,
    debug_mode: bool,
) {
    for result in csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(file)
        .deserialize::<TransactionLine>()
        .map(|line| {
            TransactionOrder::try_from(line?)?
                .process(accounts, operations_register)
                .map_err(|e| Error::from(e))
        })
    {
        match (debug_mode, result) {
            (true, Err(e)) => println!("{}", e),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    fn try_input(input: &str) -> Vec<u8> {
        let mut accounts = crate::ClientAccounts::new();
        let mut operations_register = crate::MoneyOperationsRegister::new();
        crate::read_transactions_file(
            input.as_bytes(),
            &mut accounts,
            &mut operations_register,
            false,
        );
        let mut buf = Vec::new();
        accounts.print_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn wrong_format() {
        let sample_operation = "type, 		client,	tx,	amount
        Deposit,	1.0,	1,	2.0";
        let output = try_input(&sample_operation);
        assert_eq!("", std::str::from_utf8(&output).unwrap());
    }

    #[test]
    fn precision() {
        let sample_operation = "type, 		client,	tx,	amount
        deposit,	1,	1,	2.234235";
        let output = try_input(&sample_operation);
        assert_eq!(
            "client,available,held,total,locked\n1,2.2342,0.0,2.2342,false\n",
            std::str::from_utf8(&output).unwrap()
        );
    }

    #[test]
    fn missing_reference() {
        let sample_operation = "type, 		client,	tx,	amount
        deposit,	1,	1,	2.0
        withdrawal, 2, 2, 1.0";
        let output = try_input(&sample_operation);
        assert_eq!(
            "client,available,held,total,locked\n1,2.0,0.0,2.0,false\n",
            std::str::from_utf8(&output).unwrap()
        );
    }

    #[test]
    fn not_enough_funds() {
        let sample_operation = "type, 		client,	tx,	amount
        deposit,	1,	1,	2.0
        withdrawal, 2, 2, 5.0";
        let output = try_input(&sample_operation);
        assert_eq!(
            "client,available,held,total,locked\n1,2.0,0.0,2.0,false\n",
            std::str::from_utf8(&output).unwrap()
        );
    }

    #[test]
    fn wrong_state() {
        let sample_operation = "type, 		client,	tx,	amount
        deposit,	1,	1,	2.0
        resolve, 1, 1,";
        let output = try_input(&sample_operation);
        assert_eq!(
            "client,available,held,total,locked\n1,2.0,0.0,2.0,false\n",
            std::str::from_utf8(&output).unwrap()
        );
    }
}
