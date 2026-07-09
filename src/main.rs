mod amount;
mod codec;
mod crypto;
mod error;
mod ids;
mod ledger;
mod order;
mod runtime;

pub use amount::{Amount, Bps};
pub use codec::canonical_bytes;
pub use crypto::{KeyPair, PublicIdentity, SignatureBytes, verify_signature};
pub use error::{ApexError, ApexResult};
pub use ids::{AccountId, AssetId, Digest, IntentId, TxId};
pub use ledger::{AccountState, ApexLedger, IntentState, JournalEntry, JournalOp};
pub use order::{
    IntentAuthorizationView, IntentPolicy, IntentTerms, LiquidityCredit, RoutePlan,
    SettlementAuthorizationView, SettlementRequest, SignedIntent, SignedSettlement,
};
pub use runtime::ScenarioReport;

fn main() {
    if let Err(error) = runtime::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
