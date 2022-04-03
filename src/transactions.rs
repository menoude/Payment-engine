use crate::{ClientAccounts, ClientId};
use {
    std::{collections::HashMap, fmt},
    strum_macros::Display,
};

#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, serde::Deserialize)]
pub struct TransactionId(pub u32);

#[derive(Debug)]
pub enum TransactionError {
    AlreadyExists(TransactionId),
    LockedAccount(ClientId),
    MissingClient(ClientId),
    MissingOperation(TransactionId),
    NotEnoughFunds,
    WrongTransactionState,
}

impl fmt::Display for TransactionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "{}",
            match *self {
                Self::AlreadyExists(TransactionId(id)) =>
                    format!("Transaction {} already exists", id),
                Self::LockedAccount(ClientId(client_id)) =>
                    format!("Client account {} is locked", client_id),
                Self::MissingClient(ClientId(client_id)) =>
                    format!("Can't find client {}", client_id),
                Self::MissingOperation(TransactionId(transaction_id)) =>
                    format!("Can't find transaction {}", transaction_id),
                Self::NotEnoughFunds => String::from("Not enough funds"),
                Self::WrongTransactionState => String::from("Wrong transaction state"),
            }
        )
    }
}
#[derive(Debug, Display)]
pub enum TransactionOrder {
    MoneyOperation(MoneyOperation),
    ClientClaim(ClientClaim),
}

impl TransactionOrder {
    pub fn process(
        self,
        clients_map: &mut ClientAccounts,
        operations_register: &mut MoneyOperationsRegister,
    ) -> Result<(), TransactionError> {
        match self {
            Self::MoneyOperation(money_operation) => {
                money_operation.process(clients_map, operations_register)
            }
            Self::ClientClaim(client_claim) => {
                client_claim.process(clients_map, operations_register)
            }
        }
    }
}

#[derive(Debug)]
pub struct MoneyOperation {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
    pub disputed: bool,
    pub operation_kind: OperationKind,
}

#[derive(Debug)]
pub enum OperationKind {
    Deposit(f64),
    Withdrawal(f64),
}

impl MoneyOperation {
    pub fn process(
        self,
        clients_map: &mut ClientAccounts,
        operations_register: &mut MoneyOperationsRegister,
    ) -> Result<(), TransactionError> {
        if operations_register.contains(&self.transaction_id) {
            return Err(TransactionError::AlreadyExists(self.transaction_id));
        }
        match (
            &self.operation_kind,
            clients_map.get_account(self.client_id),
        ) {
            (_, Some(client)) if client.locked => {
                return Err(TransactionError::LockedAccount(self.client_id))
            }
            (OperationKind::Withdrawal(_), None) => {
                return Err(TransactionError::MissingClient(self.client_id))
            }
            (OperationKind::Withdrawal(amount), Some(client)) => {
                if !client.has_enough_funds(*amount) {
                    return Err(TransactionError::NotEnoughFunds);
                }
                client.decrease_funds(*amount)
            }
            (OperationKind::Deposit(amount), Some(client)) => client.increase_funds(*amount),
            (OperationKind::Deposit(amount), None) => {
                clients_map.create_client(self.client_id, *amount)
            }
        }
        operations_register.insert(self.transaction_id, self);
        Ok(())
    }
}

pub struct MoneyOperationsRegister {
    inner: HashMap<TransactionId, MoneyOperation>,
}

impl MoneyOperationsRegister {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
    pub fn contains(&self, id: &TransactionId) -> bool {
        self.inner.get(&id).is_some()
    }
    pub fn get_operation(&mut self, id: TransactionId) -> Option<&mut MoneyOperation> {
        self.inner.get_mut(&id)
    }
    pub fn insert(&mut self, id: TransactionId, operation: MoneyOperation) {
        self.inner.insert(id, operation);
    }
}

#[derive(Debug)]
pub struct ClientClaim {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
    pub claim_kind: ClientClaimKind,
}

#[derive(Debug)]
pub enum ClientClaimKind {
    Resolve,
    Dispute,
    Chargeback,
}

impl ClientClaim {
    pub fn process(
        self,
        clients_map: &mut ClientAccounts,
        operations_register: &mut MoneyOperationsRegister,
    ) -> Result<(), TransactionError> {
        let (operation, client) = match (
            operations_register.get_operation(self.transaction_id),
            clients_map.get_account(self.client_id),
        ) {
            (_, Some(client)) if client.locked => {
                return Err(TransactionError::LockedAccount(self.client_id))
            }
            (Some(operation), Some(client)) => (operation, client),
            (None, _) => return Err(TransactionError::MissingOperation(self.transaction_id)),
            (_, None) => return Err(TransactionError::MissingClient(self.client_id)),
        };

        match self.claim_kind {
            ClientClaimKind::Dispute if !operation.disputed => {
                // Negative funds are accepted when it's due to disputes
                match operation.operation_kind {
                    OperationKind::Deposit(amount) => {
                        client.hold_funds(amount);
                        client.decrease_funds(amount)
                    }
                    OperationKind::Withdrawal(amount) => client.hold_funds(amount),
                }
                operation.disputed = true;
            }
            ClientClaimKind::Resolve if operation.disputed => {
                // Negative held funds is treated as an error
                match operation.operation_kind {
                    OperationKind::Deposit(amount) => {
                        client.release_funds(amount)?;
                    }
                    OperationKind::Withdrawal(amount) => client.clear_held_funds(amount)?,
                }
                operation.disputed = false;
            }
            ClientClaimKind::Chargeback if !operation.disputed => {
                match operation.operation_kind {
                    OperationKind::Deposit(amount) => {
                        client.clear_held_funds(amount)?;
                    }
                    OperationKind::Withdrawal(amount) => {
                        client.release_funds(amount)?;
                    }
                }
                operation.disputed = false;
                client.locked = true;
            }
            _ => return Err(TransactionError::WrongTransactionState),
        }
        Ok(())
    }
}
