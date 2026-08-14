mod amount;
mod checkpoint;
mod codec;
mod controls;
mod crypto;
mod economics;
mod error;
mod governance;
mod ids;
mod ledger;
mod order;
mod risk;
mod runtime;

pub use amount::{Amount, Bps};
pub use checkpoint::StateCheckpoint;
pub use codec::canonical_bytes;
pub use controls::{ControlMode, FlowGuard, FlowKind, FlowLimits, FlowRecord, FlowRequest};
pub use crypto::{KeyPair, PublicIdentity, SignatureBytes, verify_signature};
pub use economics::{EconomicInputs, EconomicQuote, FeeCurve, PricingEngine};
pub use error::{ApexError, ApexResult};
pub use governance::{
    Governance, GovernanceAction, GovernanceConfig, GovernanceMember, GovernanceRole, Proposal,
    ProposalState,
};
pub use ids::{AccountId, AssetId, Digest, IntentId, TxId};
pub use ledger::{AccountState, ApexLedger, IntentState, JournalEntry, JournalOp};
pub use order::{
    IntentAuthorizationView, IntentPolicy, IntentTerms, LiquidityCredit, RoutePlan,
    SettlementAuthorizationView, SettlementRequest, SignedIntent, SignedSettlement,
};
pub use risk::{
    AdmissionDecision, CorridorLimits, PortfolioExposure, RiskBand, RiskEngine, RiskSignals,
    RiskWeights,
};
pub use runtime::{ScenarioReport, run};

pub const VERSION: &str = "1.0.0";
