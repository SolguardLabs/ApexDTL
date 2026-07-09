use serde::Serialize;

use crate::{Amount, PublicIdentity};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AccountState {
    pub identity: PublicIdentity,
    pub balance: Amount,
    pub next_intent_nonce: u64,
    pub next_settlement_nonce: u64,
}

impl AccountState {
    pub fn new(identity: PublicIdentity) -> Self {
        Self {
            identity,
            balance: Amount::zero(),
            next_intent_nonce: 0,
            next_settlement_nonce: 0,
        }
    }
}
