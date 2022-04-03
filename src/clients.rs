use crate::TransactionError;
use {
    serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer},
    std::{collections::HashMap, io::Write},
};

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub struct ClientId(pub u32);

impl std::fmt::Display for ClientId {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

pub struct ClientAccounts {
    inner: HashMap<ClientId, Client>,
}

impl ClientAccounts {
    pub fn new() -> ClientAccounts {
        ClientAccounts {
            inner: HashMap::new(),
        }
    }
    pub fn get_account(&mut self, client_id: ClientId) -> Option<&mut Client> {
        self.inner.get_mut(&client_id)
    }
    pub fn create_client(&mut self, id: ClientId, funds: f64) {
        self.inner.insert(
            id,
            Client {
                funds,
                held_funds: 0.,
                locked: false,
            },
        );
    }
    pub fn print_to<W: Write>(&self, w: &mut W) -> Result<(), csv::Error> {
        let mut writer = csv::Writer::from_writer(w);
        for (id, account) in &self.inner {
            writer.serialize(AccountSummary {
                client: *id,
                available: account.funds,
                held: account.held_funds,
                locked: account.locked,
                total: account.held_funds + account.funds,
            })?
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Client {
    funds: f64,
    held_funds: f64,
    pub locked: bool,
}

impl Client {
    pub fn increase_funds(&mut self, amount: f64) {
        self.funds += amount;
    }
    pub fn decrease_funds(&mut self, amount: f64) {
        self.funds -= amount;
    }
    pub fn has_enough_funds(&self, amount: f64) -> bool {
        self.funds >= amount
    }
    pub fn hold_funds(&mut self, amount: f64) {
        self.funds -= amount;
        self.held_funds += amount;
    }
    pub fn clear_held_funds(&mut self, amount: f64) -> Result<(), TransactionError> {
        if self.held_funds < amount {
            return Err(TransactionError::NotEnoughFunds);
        }
        self.held_funds -= amount;
        Ok(())
    }
    pub fn release_funds(&mut self, amount: f64) -> Result<(), TransactionError> {
        if self.held_funds < amount {
            return Err(TransactionError::NotEnoughFunds);
        }
        self.held_funds -= amount;
        self.funds += amount;
        Ok(())
    }
}

#[derive(Debug)]
pub struct AccountSummary {
    client: ClientId,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

trait FourDigitsPrecision {
    fn four_digits_precision(self) -> Self;
}

impl FourDigitsPrecision for f64 {
    fn four_digits_precision(self) -> Self {
        (self * 10000.).round() / 10000.
    }
}

impl Serialize for AccountSummary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AccountSummary", 5)?;
        state.serialize_field("client", &self.client)?;
        state.serialize_field("available", &(self.available.four_digits_precision()))?;
        state.serialize_field("held", &self.held.four_digits_precision())?;
        state.serialize_field("total", &self.total.four_digits_precision())?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}
