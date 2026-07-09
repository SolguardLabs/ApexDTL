use serde::Serialize;

use crate::{AccountId, Amount, IntentId, TxId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum JournalOp {
    GenesisCredit {
        account: AccountId,
        amount: Amount,
    },
    IntentDebit {
        account: AccountId,
        intent_id: IntentId,
        amount: Amount,
    },
    IntentSettlement {
        intent_id: IntentId,
        beneficiary: AccountId,
        beneficiary_amount: Amount,
        fee_recipient: AccountId,
        fee_amount: Amount,
        rebate_recipient: AccountId,
        rebate_amount: Amount,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JournalEntry {
    pub tx_id: TxId,
    pub op: JournalOp,
}
