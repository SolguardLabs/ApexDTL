use thiserror::Error;

use crate::{AccountId, Amount, AssetId, Digest, IntentId, TxId};

pub type ApexResult<T> = Result<T, ApexError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ApexError {
    #[error("amount overflow")]
    AmountOverflow,
    #[error("amount underflow")]
    AmountUnderflow,
    #[error("zero amount")]
    ZeroAmount,
    #[error("basis points out of range: {0}")]
    BpsOutOfRange(u16),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("signature error: {0}")]
    Signature(String),
    #[error("account already exists: {0}")]
    AccountAlreadyExists(AccountId),
    #[error("account not found: {0}")]
    AccountNotFound(AccountId),
    #[error("intent already exists: {0}")]
    IntentAlreadyExists(IntentId),
    #[error("intent not found: {0}")]
    IntentNotFound(IntentId),
    #[error("intent already settled: {0}")]
    IntentSettled(IntentId),
    #[error("duplicate transaction: {0}")]
    DuplicateTransaction(TxId),
    #[error("insufficient funds for {account}: available {available}, required {required}")]
    InsufficientFunds {
        account: AccountId,
        available: Amount,
        required: Amount,
    },
    #[error("asset mismatch: expected {expected}, received {received}")]
    AssetMismatch {
        expected: AssetId,
        received: AssetId,
    },
    #[error("nonce mismatch for {account}: expected {expected}, received {received}")]
    NonceMismatch {
        account: AccountId,
        expected: u64,
        received: u64,
    },
    #[error("unauthorized intent signer: expected {expected}, received {received}")]
    UnauthorizedIntentSigner {
        expected: AccountId,
        received: AccountId,
    },
    #[error("unauthorized settlement signer: expected {expected}, received {received}")]
    UnauthorizedSettlementSigner {
        expected: AccountId,
        received: AccountId,
    },
    #[error("route digest mismatch for {intent_id}: expected {expected}, received {received}")]
    RouteDigestMismatch {
        intent_id: IntentId,
        expected: Digest,
        received: Digest,
    },
    #[error("nonce overflow")]
    NonceOverflow,
    #[error("policy violation: {0}")]
    Policy(String),
    #[error("conservation error for {asset}: expected {expected}, observed {observed}")]
    Conservation {
        asset: AssetId,
        expected: Amount,
        observed: Amount,
    },
}
