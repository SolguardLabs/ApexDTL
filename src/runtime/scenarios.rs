use serde::Serialize;

use crate::{
    AccountId, Amount, ApexLedger, ApexResult, AssetId, Bps, Digest, EconomicInputs, FeeCurve,
    IntentPolicy, IntentTerms, KeyPair, LiquidityCredit, PricingEngine, RoutePlan,
    SettlementRequest, SignedIntent, SignedSettlement, StateCheckpoint, VERSION,
};

#[derive(Serialize)]
pub struct ScenarioReport {
    pub scenario: &'static str,
    pub network_id: u32,
    pub asset: AssetId,
    pub intent_id: Option<crate::IntentId>,
    pub open_tx: Option<crate::TxId>,
    pub settlement_tx: Option<crate::TxId>,
    pub balances: BalanceReport,
    pub total_supply: Amount,
    pub state_digest: Digest,
    pub conservation_ok: bool,
}

#[derive(Serialize)]
pub struct BalanceReport {
    pub payer: Amount,
    pub beneficiary: Amount,
    pub solver: Amount,
    pub integrator: Amount,
    pub reserve: Amount,
    pub sponsor: Amount,
}

#[derive(Serialize)]
pub struct QuoteScenarioReport {
    pub scenario: &'static str,
    pub version: &'static str,
    pub quote: crate::EconomicQuote,
}

#[derive(Serialize)]
pub struct CheckpointScenarioReport {
    pub scenario: &'static str,
    pub version: &'static str,
    pub checkpoint: StateCheckpoint,
    pub account_count: usize,
    pub intent_count: usize,
}

struct Fixture {
    ledger: ApexLedger,
    payer: KeyPair,
    beneficiary: KeyPair,
    solver: KeyPair,
    integrator: KeyPair,
    reserve: KeyPair,
    sponsor: KeyPair,
    network_id: u32,
    asset: AssetId,
}

impl Fixture {
    fn new() -> ApexResult<Self> {
        let network_id = 9_042;
        let asset = AssetId::native();
        let payer = KeyPair::from_seed([1u8; 32]);
        let beneficiary = KeyPair::from_seed([2u8; 32]);
        let solver = KeyPair::from_seed([3u8; 32]);
        let integrator = KeyPair::from_seed([4u8; 32]);
        let reserve = KeyPair::from_seed([5u8; 32]);
        let sponsor = KeyPair::from_seed([6u8; 32]);
        let mut ledger = ApexLedger::new(network_id, asset);

        for identity in [
            payer.public_identity(),
            beneficiary.public_identity(),
            solver.public_identity(),
            integrator.public_identity(),
            reserve.public_identity(),
            sponsor.public_identity(),
        ] {
            ledger.register_account(identity)?;
        }

        ledger.credit_genesis(payer.public_identity().account, Amount::new(10_000)?)?;

        Ok(Self {
            ledger,
            payer,
            beneficiary,
            solver,
            integrator,
            reserve,
            sponsor,
            network_id,
            asset,
        })
    }

    fn policy(&self, max_charge_bps: u16, lane: u16) -> ApexResult<IntentPolicy> {
        Ok(IntentPolicy::new(
            Digest::from_parts("apex-primary-venue-v1", &[&lane.to_be_bytes()]),
            Bps::new(max_charge_bps)?,
            lane,
            0,
            1_000,
        ))
    }

    fn balances(&self) -> ApexResult<BalanceReport> {
        Ok(BalanceReport {
            payer: self
                .ledger
                .balance_of(self.payer.public_identity().account)?,
            beneficiary: self
                .ledger
                .balance_of(self.beneficiary.public_identity().account)?,
            solver: self
                .ledger
                .balance_of(self.solver.public_identity().account)?,
            integrator: self
                .ledger
                .balance_of(self.integrator.public_identity().account)?,
            reserve: self
                .ledger
                .balance_of(self.reserve.public_identity().account)?,
            sponsor: self
                .ledger
                .balance_of(self.sponsor.public_identity().account)?,
        })
    }

    fn report(
        &self,
        scenario: &'static str,
        intent_id: Option<crate::IntentId>,
        open_tx: Option<crate::TxId>,
        settlement_tx: Option<crate::TxId>,
    ) -> ApexResult<ScenarioReport> {
        Ok(ScenarioReport {
            scenario,
            network_id: self.network_id,
            asset: self.asset,
            intent_id,
            open_tx,
            settlement_tx,
            balances: self.balances()?,
            total_supply: self.ledger.total_supply(),
            state_digest: self.ledger.state_digest()?,
            conservation_ok: self.ledger.verify_conservation().is_ok(),
        })
    }
}

pub fn direct() -> ApexResult<ScenarioReport> {
    let mut fixture = Fixture::new()?;
    let lane = 1;
    let policy = fixture.policy(25, lane)?;
    let route = RoutePlan::direct(
        fixture.solver.public_identity().account,
        policy.venue,
        lane,
        0,
    );
    open_and_settle(&mut fixture, "direct", Amount::new(900)?, policy, route)
}

pub fn routed() -> ApexResult<ScenarioReport> {
    let mut fixture = Fixture::new()?;
    let lane = 2;
    let policy = fixture.policy(150, lane)?;
    let route = RoutePlan {
        venue: policy.venue,
        solver: fixture.solver.public_identity().account,
        fee_recipient: fixture.solver.public_identity().account,
        operator_fee: Amount::new(12)?,
        rebate_recipient: Some(fixture.integrator.public_identity().account),
        rebate_amount: Amount::new(3)?,
        credit: LiquidityCredit::none(fixture.sponsor.public_identity().account),
        execution_lane: lane,
        quote_nonce: 7,
    };
    open_and_settle(&mut fixture, "routed", Amount::new(1_200)?, policy, route)
}

pub fn batch() -> ApexResult<ScenarioReport> {
    let mut fixture = Fixture::new()?;
    let lane = 3;
    let policy = fixture.policy(200, lane)?;
    let first_route = RoutePlan {
        venue: policy.venue,
        solver: fixture.solver.public_identity().account,
        fee_recipient: fixture.solver.public_identity().account,
        operator_fee: Amount::new(8)?,
        rebate_recipient: Some(fixture.integrator.public_identity().account),
        rebate_amount: Amount::new(2)?,
        credit: LiquidityCredit::none(fixture.sponsor.public_identity().account),
        execution_lane: lane,
        quote_nonce: 11,
    };
    let first = open_and_settle_internal(&mut fixture, Amount::new(500)?, policy, first_route)?;

    let second_route = RoutePlan {
        venue: policy.venue,
        solver: fixture.solver.public_identity().account,
        fee_recipient: fixture.reserve.public_identity().account,
        operator_fee: Amount::new(6)?,
        rebate_recipient: Some(fixture.integrator.public_identity().account),
        rebate_amount: Amount::new(4)?,
        credit: LiquidityCredit {
            provider: fixture.sponsor.public_identity().account,
            amount: Amount::new(2)?,
            reference: Digest::from_parts("apex-credit-reference-v1", &[b"batch-two"]),
        },
        execution_lane: lane,
        quote_nonce: 12,
    };
    let _second = open_and_settle_internal(&mut fixture, Amount::new(700)?, policy, second_route)?;

    fixture.report("batch", Some(first.0), Some(first.1), Some(first.2))
}

pub fn snapshot() -> ApexResult<ScenarioReport> {
    let fixture = Fixture::new()?;
    fixture.report("snapshot", None, None, None)
}

pub fn quote() -> ApexResult<QuoteScenarioReport> {
    let quote = PricingEngine::new(FeeCurve::default())?.quote(EconomicInputs {
        principal: Amount::new(10_000_000)?,
        utilization_bps: Bps::new(9_000)?,
        finality_epochs: 6,
        loss_probability_ppm: 2_000,
        loss_severity_bps: Bps::new(4_000)?,
        risk_score_bps: Bps::new(4_000)?,
    })?;
    Ok(QuoteScenarioReport {
        scenario: "quote",
        version: VERSION,
        quote,
    })
}

pub fn checkpoint() -> ApexResult<CheckpointScenarioReport> {
    let mut fixture = Fixture::new()?;
    fixture.ledger.set_epoch(12)?;
    let checkpoint = StateCheckpoint::build(&fixture.ledger, 1, None)?;
    Ok(CheckpointScenarioReport {
        scenario: "checkpoint",
        version: VERSION,
        checkpoint,
        account_count: fixture.ledger.account_count(),
        intent_count: fixture.ledger.intent_count(),
    })
}

fn open_and_settle(
    fixture: &mut Fixture,
    scenario: &'static str,
    amount: Amount,
    policy: IntentPolicy,
    route: RoutePlan,
) -> ApexResult<ScenarioReport> {
    let (intent_id, open_tx, settlement_tx) =
        open_and_settle_internal(fixture, amount, policy, route)?;
    fixture.report(
        scenario,
        Some(intent_id),
        Some(open_tx),
        Some(settlement_tx),
    )
}

fn open_and_settle_internal(
    fixture: &mut Fixture,
    amount: Amount,
    policy: IntentPolicy,
    route: RoutePlan,
) -> ApexResult<(crate::IntentId, crate::TxId, crate::TxId)> {
    let payer = fixture.payer.public_identity().account;
    let beneficiary = fixture.beneficiary.public_identity().account;
    let payer_nonce = fixture.ledger.intent_nonce(payer)?;
    let salt = Digest::from_parts(
        "apex-intent-salt-v1",
        &[&payer_nonce.to_be_bytes(), &route.quote_nonce.to_be_bytes()],
    );
    let terms = IntentTerms::new(
        fixture.network_id,
        payer,
        beneficiary,
        fixture.asset,
        amount,
        payer_nonce,
        policy,
        salt,
    )?;
    let signed_intent = SignedIntent::sign(terms, route, &fixture.payer)?;
    let open_tx = fixture.ledger.open_intent(&signed_intent)?;
    let route_digest = route.route_digest()?;
    let request = SettlementRequest::new(
        fixture.network_id,
        terms.intent_id,
        beneficiary,
        fixture.ledger.settlement_nonce(beneficiary)?,
        route_digest,
    );
    let signed_settlement = SignedSettlement::sign(request, &fixture.beneficiary)?;
    let settlement_tx = fixture.ledger.settle_intent(&signed_settlement)?;
    Ok((terms.intent_id, open_tx, settlement_tx))
}

#[allow(dead_code)]
fn account_from_seed(seed: u8) -> AccountId {
    KeyPair::from_seed([seed; 32]).public_identity().account
}
