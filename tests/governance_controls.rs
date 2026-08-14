use std::collections::BTreeSet;

use apex_dtl::{
    Amount, ApexError, ApexResult, ControlMode, Digest, FlowGuard, FlowKind, FlowLimits,
    FlowRequest, Governance, GovernanceAction, GovernanceConfig, GovernanceMember, GovernanceRole,
    KeyPair, ProposalState,
};

struct Actors {
    risk_a: KeyPair,
    risk_b: KeyPair,
    guardian: KeyPair,
    executor: KeyPair,
}

fn governance() -> ApexResult<(Governance, Actors)> {
    let actors = Actors {
        risk_a: KeyPair::from_seed([51u8; 32]),
        risk_b: KeyPair::from_seed([52u8; 32]),
        guardian: KeyPair::from_seed([53u8; 32]),
        executor: KeyPair::from_seed([54u8; 32]),
    };
    let governance = Governance::new(
        GovernanceConfig {
            quorum_weight: 70,
            timelock_epochs: 3,
            expiry_epochs: 20,
        },
        [
            GovernanceMember {
                account: actors.risk_a.public_identity().account,
                role: GovernanceRole::RiskCouncil,
                weight: 45,
                active: true,
            },
            GovernanceMember {
                account: actors.risk_b.public_identity().account,
                role: GovernanceRole::RiskCouncil,
                weight: 35,
                active: true,
            },
            GovernanceMember {
                account: actors.guardian.public_identity().account,
                role: GovernanceRole::Guardian,
                weight: 40,
                active: true,
            },
            GovernanceMember {
                account: actors.executor.public_identity().account,
                role: GovernanceRole::Executor,
                weight: 1,
                active: true,
            },
        ],
    )?;
    Ok((governance, actors))
}

#[test]
fn governance_enforces_quorum_and_timelock() -> ApexResult<()> {
    let (mut governance, actors) = governance()?;
    let proposal = governance.submit(
        actors.risk_a.public_identity().account,
        GovernanceAction::SetRiskProfile {
            corridor: Digest::from_parts("corridor", &[b"eth-sol"]),
            payload_digest: Digest::from_parts("payload", &[b"risk-v2"]),
        },
        Digest::from_parts("rationale", &[b"quarterly-review"]),
        10,
    )?;
    governance.approve(proposal.id, actors.risk_a.public_identity().account, 11)?;
    let approved = governance.approve(proposal.id, actors.risk_b.public_identity().account, 12)?;
    assert_eq!(approved.approval_weight, 80);
    let queued = governance.queue(proposal.id, 12)?;
    assert_eq!(queued.state, ProposalState::Queued);
    assert_eq!(queued.execute_after_epoch, Some(15));
    assert!(matches!(
        governance.execute(proposal.id, actors.executor.public_identity().account, 14),
        Err(ApexError::State(_))
    ));
    governance.execute(proposal.id, actors.executor.public_identity().account, 15)?;
    Ok(())
}

#[test]
fn governance_rejects_retroactive_approval_and_queue() -> ApexResult<()> {
    let (mut governance, actors) = governance()?;
    let proposal = governance.submit(
        actors.risk_a.public_identity().account,
        GovernanceAction::SetFeeCurve {
            payload_digest: Digest::from_parts("payload", &[b"fee-v2"]),
        },
        Digest::from_parts("rationale", &[b"market-review"]),
        20,
    )?;
    assert!(matches!(
        governance.approve(proposal.id, actors.risk_a.public_identity().account, 19),
        Err(ApexError::State(_))
    ));
    governance.approve(proposal.id, actors.risk_a.public_identity().account, 21)?;
    governance.approve(proposal.id, actors.risk_b.public_identity().account, 22)?;
    assert!(matches!(
        governance.queue(proposal.id, 21),
        Err(ApexError::State(_))
    ));
    governance.queue(proposal.id, 22)?;
    Ok(())
}

fn flow_guard(actors: &Actors) -> ApexResult<FlowGuard> {
    FlowGuard::new(FlowLimits {
        window_epochs: 10,
        max_single: Amount::new(500_000)?,
        max_window: Amount::new(1_000_000)?,
        restricted_max_single: Amount::new(100_000)?,
        large_operation_threshold: Amount::new(250_000)?,
        large_operation_approvals: 2,
        authorized_approvers: BTreeSet::from([
            actors.risk_a.public_identity().account,
            actors.risk_b.public_identity().account,
            actors.guardian.public_identity().account,
        ]),
    })
}

#[test]
fn flow_guard_enforces_authorized_approvers_and_window() -> ApexResult<()> {
    let (_, actors) = governance()?;
    let mut guard = flow_guard(&actors)?;
    let approvers = vec![
        actors.risk_a.public_identity().account,
        actors.risk_b.public_identity().account,
    ];
    guard.authorize(FlowRequest {
        id: Digest::from_parts("flow", &[b"one"]),
        kind: FlowKind::Settlement,
        subject: Digest::from_parts("route", &[b"eth-sol"]),
        amount: Amount::new(300_000)?,
        epoch: 10,
        approvers: approvers.clone(),
    })?;
    guard.authorize(FlowRequest {
        id: Digest::from_parts("flow", &[b"two"]),
        kind: FlowKind::Settlement,
        subject: Digest::from_parts("route", &[b"eth-sol"]),
        amount: Amount::new(500_000)?,
        epoch: 12,
        approvers: approvers.clone(),
    })?;
    assert!(matches!(
        guard.authorize(FlowRequest {
            id: Digest::from_parts("flow", &[b"three"]),
            kind: FlowKind::Settlement,
            subject: Digest::from_parts("route", &[b"eth-sol"]),
            amount: Amount::new(250_000)?,
            epoch: 13,
            approvers,
        }),
        Err(ApexError::State(_))
    ));
    assert_eq!(guard.records().count(), 2);
    Ok(())
}

#[test]
fn halted_mode_keeps_only_protective_penalties_available() -> ApexResult<()> {
    let (_, actors) = governance()?;
    let mut guard = flow_guard(&actors)?;
    guard.set_mode(ControlMode::Halted);
    let settlement = FlowRequest {
        id: Digest::from_parts("flow", &[b"halted-settlement"]),
        kind: FlowKind::Settlement,
        subject: Digest::from_parts("route", &[b"eth-sol"]),
        amount: Amount::new(50_000)?,
        epoch: 20,
        approvers: vec![],
    };
    assert!(matches!(
        guard.authorize(settlement),
        Err(ApexError::State(_))
    ));
    guard.authorize(FlowRequest {
        id: Digest::from_parts("flow", &[b"halted-penalty"]),
        kind: FlowKind::Penalty,
        subject: Digest::from_parts("route", &[b"eth-sol"]),
        amount: Amount::new(50_000)?,
        epoch: 20,
        approvers: vec![],
    })?;
    Ok(())
}

#[test]
fn flow_guard_rejects_retroactive_requests() -> ApexResult<()> {
    let (_, actors) = governance()?;
    let mut guard = flow_guard(&actors)?;
    guard.authorize(FlowRequest {
        id: Digest::from_parts("flow", &[b"current"]),
        kind: FlowKind::Settlement,
        subject: Digest::from_parts("route", &[b"eth-sol"]),
        amount: Amount::new(50_000)?,
        epoch: 20,
        approvers: vec![],
    })?;
    assert!(matches!(
        guard.authorize(FlowRequest {
            id: Digest::from_parts("flow", &[b"retroactive"]),
            kind: FlowKind::Settlement,
            subject: Digest::from_parts("route", &[b"eth-sol"]),
            amount: Amount::new(50_000)?,
            epoch: 19,
            approvers: vec![],
        }),
        Err(ApexError::State(_))
    ));
    assert_eq!(guard.records().count(), 1);
    Ok(())
}
