use apex_dtl::{
    Amount, ApexError, ApexLedger, ApexResult, AssetId, Bps, Digest, IntentPolicy, IntentTerms,
    KeyPair, RoutePlan, SignedIntent,
};

fn registered_ledger() -> ApexResult<(ApexLedger, KeyPair, KeyPair, KeyPair)> {
    let payer = KeyPair::from_seed([31u8; 32]);
    let beneficiary = KeyPair::from_seed([32u8; 32]);
    let solver = KeyPair::from_seed([33u8; 32]);
    let mut ledger = ApexLedger::new(77, AssetId::native());
    for identity in [
        payer.public_identity(),
        beneficiary.public_identity(),
        solver.public_identity(),
    ] {
        ledger.register_account(identity)?;
    }
    ledger.credit_genesis(payer.public_identity().account, Amount::new(10_000)?)?;
    Ok((ledger, payer, beneficiary, solver))
}

#[test]
fn route_rejects_execution_venue_mismatch() -> ApexResult<()> {
    let solver = KeyPair::from_seed([41u8; 32]);
    let allowed_venue = Digest::from_parts("venue", &[b"allowed"]);
    let different_venue = Digest::from_parts("venue", &[b"different"]);
    let route = RoutePlan::direct(solver.public_identity().account, different_venue, 9, 1);
    let policy = IntentPolicy::new(allowed_venue, Bps::new(100)?, 9, 0, 100);
    let error = route
        .validate(policy, Amount::new(1_000)?)
        .expect_err("venue mismatch must be rejected");
    assert!(matches!(error, ApexError::Policy(message) if message.contains("venue")));
    Ok(())
}

#[test]
fn ledger_enforces_intent_validity_window() -> ApexResult<()> {
    let (mut ledger, payer, beneficiary, solver) = registered_ledger()?;
    ledger.set_epoch(12)?;
    let venue = Digest::from_parts("venue", &[b"primary"]);
    let policy = IntentPolicy::new(venue, Bps::new(100)?, 4, 20, 40);
    let route = RoutePlan::direct(solver.public_identity().account, venue, 4, 1);
    let terms = IntentTerms::new(
        ledger.network_id(),
        payer.public_identity().account,
        beneficiary.public_identity().account,
        ledger.asset(),
        Amount::new(1_000)?,
        ledger.intent_nonce(payer.public_identity().account)?,
        policy,
        Digest::from_parts("intent-salt", &[b"window"]),
    )?;
    let signed = SignedIntent::sign(terms, route, &payer)?;
    assert!(matches!(
        ledger.open_intent(&signed),
        Err(ApexError::Policy(_))
    ));

    ledger.set_epoch(20)?;
    ledger.open_intent(&signed)?;
    assert_eq!(ledger.intent_count(), 1);
    Ok(())
}

#[test]
fn ledger_rejects_inverted_validity_window() -> ApexResult<()> {
    let (mut ledger, payer, beneficiary, solver) = registered_ledger()?;
    ledger.set_epoch(30)?;
    let venue = Digest::from_parts("venue", &[b"primary"]);
    let policy = IntentPolicy::new(venue, Bps::new(100)?, 4, 40, 20);
    let route = RoutePlan::direct(solver.public_identity().account, venue, 4, 2);
    let terms = IntentTerms::new(
        ledger.network_id(),
        payer.public_identity().account,
        beneficiary.public_identity().account,
        ledger.asset(),
        Amount::new(1_000)?,
        ledger.intent_nonce(payer.public_identity().account)?,
        policy,
        Digest::from_parts("intent-salt", &[b"inverted"]),
    )?;
    let signed = SignedIntent::sign(terms, route, &payer)?;
    assert!(matches!(
        ledger.open_intent(&signed),
        Err(ApexError::Policy(_))
    ));
    assert_eq!(
        ledger.balance_of(payer.public_identity().account)?.units(),
        10_000
    );
    Ok(())
}

#[test]
fn ledger_epoch_is_monotonic() -> ApexResult<()> {
    let (mut ledger, _, _, _) = registered_ledger()?;
    ledger.set_epoch(12)?;
    assert!(matches!(ledger.set_epoch(11), Err(ApexError::State(_))));
    assert_eq!(ledger.current_epoch(), 12);
    Ok(())
}
