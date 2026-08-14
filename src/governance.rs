use std::collections::BTreeMap;

use serde::Serialize;

use crate::{AccountId, ApexError, ApexResult, Digest};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceRole {
    RiskCouncil,
    Operations,
    Guardian,
    Executor,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GovernanceMember {
    pub account: AccountId,
    pub role: GovernanceRole,
    pub weight: u64,
    pub active: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceAction {
    SetRiskProfile {
        corridor: Digest,
        payload_digest: Digest,
    },
    SetFeeCurve {
        payload_digest: Digest,
    },
    PauseCorridor {
        corridor: Digest,
    },
    ResumeCorridor {
        corridor: Digest,
    },
    RotateMember {
        account: AccountId,
        payload_digest: Digest,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    Open,
    Queued,
    Executed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Proposal {
    pub id: Digest,
    pub proposer: AccountId,
    pub action: GovernanceAction,
    pub rationale_digest: Digest,
    pub created_at_epoch: u64,
    pub execute_after_epoch: Option<u64>,
    pub executed_at_epoch: Option<u64>,
    pub state: ProposalState,
    pub approvals: BTreeMap<AccountId, u64>,
    pub approval_weight: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GovernanceConfig {
    pub quorum_weight: u64,
    pub timelock_epochs: u64,
    pub expiry_epochs: u64,
}

impl GovernanceConfig {
    pub fn validate(self) -> ApexResult<()> {
        if self.quorum_weight == 0 {
            return Err(ApexError::InvalidConfiguration(
                "governance quorum must be positive".to_owned(),
            ));
        }
        if self.timelock_epochs == 0 || self.expiry_epochs <= self.timelock_epochs {
            return Err(ApexError::InvalidConfiguration(
                "governance expiry must exceed a positive timelock".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Governance {
    config: GovernanceConfig,
    members: BTreeMap<AccountId, GovernanceMember>,
    proposals: BTreeMap<Digest, Proposal>,
}

impl Governance {
    pub fn new(
        config: GovernanceConfig,
        members: impl IntoIterator<Item = GovernanceMember>,
    ) -> ApexResult<Self> {
        config.validate()?;
        let mut member_map = BTreeMap::new();
        for member in members {
            if member.weight == 0 {
                return Err(ApexError::InvalidConfiguration(
                    "governance member weight must be positive".to_owned(),
                ));
            }
            if member_map.insert(member.account, member).is_some() {
                return Err(ApexError::InvalidConfiguration(
                    "governance member is duplicated".to_owned(),
                ));
            }
        }
        let mut approval_weight = 0u64;
        let mut has_guardian = false;
        let mut has_executor = false;
        for member in member_map.values().filter(|member| member.active) {
            if matches!(
                member.role,
                GovernanceRole::RiskCouncil | GovernanceRole::Guardian
            ) {
                approval_weight = approval_weight
                    .checked_add(member.weight)
                    .ok_or(ApexError::AmountOverflow)?;
            }
            has_guardian |= member.role == GovernanceRole::Guardian;
            has_executor |= member.role == GovernanceRole::Executor;
        }
        if approval_weight < config.quorum_weight || !has_guardian || !has_executor {
            return Err(ApexError::InvalidConfiguration(
                "governance quorum or mandatory roles are unavailable".to_owned(),
            ));
        }
        Ok(Self {
            config,
            members: member_map,
            proposals: BTreeMap::new(),
        })
    }

    pub fn submit(
        &mut self,
        proposer: AccountId,
        action: GovernanceAction,
        rationale_digest: Digest,
        epoch: u64,
    ) -> ApexResult<Proposal> {
        let member = self.active_member(proposer)?;
        if !matches!(
            member.role,
            GovernanceRole::RiskCouncil | GovernanceRole::Operations
        ) {
            return Err(ApexError::Unauthorized(
                "member cannot submit governance proposals".to_owned(),
            ));
        }
        let id = Digest::from_serializable(
            "apex-governance-proposal-v1",
            &(proposer, action, rationale_digest, epoch),
        )?;
        if self.proposals.contains_key(&id) {
            return Err(ApexError::State(
                "governance proposal already exists".to_owned(),
            ));
        }
        let proposal = Proposal {
            id,
            proposer,
            action,
            rationale_digest,
            created_at_epoch: epoch,
            execute_after_epoch: None,
            executed_at_epoch: None,
            state: ProposalState::Open,
            approvals: BTreeMap::new(),
            approval_weight: 0,
        };
        self.proposals.insert(id, proposal.clone());
        Ok(proposal)
    }

    pub fn config(&self) -> GovernanceConfig {
        self.config
    }

    pub fn approve(
        &mut self,
        proposal_id: Digest,
        approver: AccountId,
        epoch: u64,
    ) -> ApexResult<Proposal> {
        let member = *self.active_member(approver)?;
        if !matches!(
            member.role,
            GovernanceRole::RiskCouncil | GovernanceRole::Guardian
        ) {
            return Err(ApexError::Unauthorized(
                "member cannot approve governance proposals".to_owned(),
            ));
        }
        let expiry = self.config.expiry_epochs;
        let proposal = self.proposal_mut(proposal_id)?;
        if proposal.state != ProposalState::Open {
            return Err(ApexError::State("proposal is not open".to_owned()));
        }
        if epoch < proposal.created_at_epoch {
            return Err(ApexError::State(
                "approval epoch precedes proposal creation".to_owned(),
            ));
        }
        if epoch > proposal.created_at_epoch.saturating_add(expiry) {
            return Err(ApexError::State(
                "proposal approval window expired".to_owned(),
            ));
        }
        if proposal.approvals.insert(approver, epoch).is_some() {
            return Err(ApexError::State(
                "member already approved proposal".to_owned(),
            ));
        }
        proposal.approval_weight = proposal
            .approval_weight
            .checked_add(member.weight)
            .ok_or(ApexError::AmountOverflow)?;
        Ok(proposal.clone())
    }

    pub fn queue(&mut self, proposal_id: Digest, epoch: u64) -> ApexResult<Proposal> {
        let config = self.config;
        let proposal = self.proposal_mut(proposal_id)?;
        if proposal.state != ProposalState::Open {
            return Err(ApexError::State("proposal is not open".to_owned()));
        }
        if proposal.approval_weight < config.quorum_weight {
            return Err(ApexError::State(
                "proposal has not reached quorum".to_owned(),
            ));
        }
        let latest_approval = proposal
            .approvals
            .values()
            .copied()
            .max()
            .unwrap_or(proposal.created_at_epoch);
        if epoch < latest_approval {
            return Err(ApexError::State(
                "queue epoch precedes proposal approvals".to_owned(),
            ));
        }
        if epoch
            > proposal
                .created_at_epoch
                .saturating_add(config.expiry_epochs)
        {
            return Err(ApexError::State("proposal expired".to_owned()));
        }
        proposal.state = ProposalState::Queued;
        proposal.execute_after_epoch = Some(
            epoch
                .checked_add(config.timelock_epochs)
                .ok_or(ApexError::EpochOverflow)?,
        );
        Ok(proposal.clone())
    }

    pub fn execute(
        &mut self,
        proposal_id: Digest,
        executor: AccountId,
        epoch: u64,
    ) -> ApexResult<GovernanceAction> {
        let member = self.active_member(executor)?;
        if member.role != GovernanceRole::Executor {
            return Err(ApexError::Unauthorized(
                "member is not a governance executor".to_owned(),
            ));
        }
        let proposal = self.proposal_mut(proposal_id)?;
        if proposal.state != ProposalState::Queued
            || epoch < proposal.execute_after_epoch.unwrap_or(u64::MAX)
        {
            return Err(ApexError::State("proposal is not executable".to_owned()));
        }
        proposal.state = ProposalState::Executed;
        proposal.executed_at_epoch = Some(epoch);
        Ok(proposal.action)
    }

    pub fn cancel(&mut self, proposal_id: Digest, guardian: AccountId) -> ApexResult<Proposal> {
        let member = self.active_member(guardian)?;
        if member.role != GovernanceRole::Guardian {
            return Err(ApexError::Unauthorized(
                "member is not a governance guardian".to_owned(),
            ));
        }
        let proposal = self.proposal_mut(proposal_id)?;
        if matches!(
            proposal.state,
            ProposalState::Executed | ProposalState::Cancelled
        ) {
            return Err(ApexError::State("proposal is terminal".to_owned()));
        }
        proposal.state = ProposalState::Cancelled;
        Ok(proposal.clone())
    }

    pub fn proposals(&self) -> impl Iterator<Item = &Proposal> {
        self.proposals.values()
    }

    fn active_member(&self, account: AccountId) -> ApexResult<&GovernanceMember> {
        self.members
            .get(&account)
            .filter(|member| member.active)
            .ok_or_else(|| ApexError::Unauthorized("active governance member not found".to_owned()))
    }

    fn proposal_mut(&mut self, id: Digest) -> ApexResult<&mut Proposal> {
        self.proposals
            .get_mut(&id)
            .ok_or_else(|| ApexError::State("governance proposal not found".to_owned()))
    }
}
