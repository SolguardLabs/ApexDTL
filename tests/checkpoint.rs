use apex_dtl::{Amount, ApexError, ApexLedger, ApexResult, AssetId, KeyPair, StateCheckpoint};

#[test]
fn checkpoint_commits_to_state_journal_epoch_and_version() -> ApexResult<()> {
    let owner = KeyPair::from_seed([61u8; 32]);
    let mut ledger = ApexLedger::new(91, AssetId::native());
    ledger.register_account(owner.public_identity())?;
    ledger.credit_genesis(owner.public_identity().account, Amount::new(1_000_000)?)?;
    ledger.set_epoch(7)?;
    let checkpoint = StateCheckpoint::build(&ledger, 1, None)?;
    assert_eq!(checkpoint.sequence, 1);
    assert_eq!(checkpoint.epoch, 7);
    assert_eq!(checkpoint.version, "1.0.0");
    checkpoint.verify(&ledger)?;

    ledger.advance_epoch(1)?;
    assert!(matches!(
        checkpoint.verify(&ledger),
        Err(ApexError::State(_))
    ));
    let next = StateCheckpoint::build(&ledger, 2, Some(checkpoint.checkpoint_digest))?;
    assert_eq!(next.previous_digest, Some(checkpoint.checkpoint_digest));
    Ok(())
}
