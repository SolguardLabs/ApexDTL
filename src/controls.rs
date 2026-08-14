use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{AccountId, Amount, ApexError, ApexResult, Digest};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlMode {
    Normal,
    Restricted,
    Halted,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowKind {
    Settlement,
    Penalty,
    Release,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FlowLimits {
    pub window_epochs: u64,
    pub max_single: Amount,
    pub max_window: Amount,
    pub restricted_max_single: Amount,
    pub large_operation_threshold: Amount,
    pub large_operation_approvals: usize,
    pub authorized_approvers: BTreeSet<AccountId>,
}

impl FlowLimits {
    pub fn validate(&self) -> ApexResult<()> {
        if self.window_epochs == 0 || self.large_operation_approvals == 0 {
            return Err(ApexError::InvalidConfiguration(
                "flow window and approval threshold must be positive".to_owned(),
            ));
        }
        if [
            self.max_single,
            self.max_window,
            self.restricted_max_single,
            self.large_operation_threshold,
        ]
        .into_iter()
        .any(Amount::is_zero)
        {
            return Err(ApexError::InvalidConfiguration(
                "flow limits must be positive".to_owned(),
            ));
        }
        if self.max_window < self.max_single || self.max_single < self.restricted_max_single {
            return Err(ApexError::InvalidConfiguration(
                "flow limits are not monotonic".to_owned(),
            ));
        }
        if self.authorized_approvers.len() < self.large_operation_approvals {
            return Err(ApexError::InvalidConfiguration(
                "authorized approvers cannot satisfy threshold".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FlowRequest {
    pub id: Digest,
    pub kind: FlowKind,
    pub subject: Digest,
    pub amount: Amount,
    pub epoch: u64,
    pub approvers: Vec<AccountId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FlowRecord {
    pub request: FlowRequest,
    pub mode: ControlMode,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FlowGuard {
    limits: FlowLimits,
    mode: ControlMode,
    latest_epoch: Option<u64>,
    records: BTreeMap<Digest, FlowRecord>,
}

impl FlowGuard {
    pub fn new(limits: FlowLimits) -> ApexResult<Self> {
        limits.validate()?;
        Ok(Self {
            limits,
            mode: ControlMode::Normal,
            latest_epoch: None,
            records: BTreeMap::new(),
        })
    }

    pub fn set_mode(&mut self, mode: ControlMode) {
        self.mode = mode;
    }

    pub fn limits(&self) -> &FlowLimits {
        &self.limits
    }

    pub fn mode(&self) -> ControlMode {
        self.mode
    }

    pub fn authorize(&mut self, request: FlowRequest) -> ApexResult<FlowRecord> {
        if request.amount.is_zero() {
            return Err(ApexError::ZeroAmount);
        }
        if self.records.contains_key(&request.id) {
            return Err(ApexError::State(
                "flow request was already consumed".to_owned(),
            ));
        }
        if self
            .latest_epoch
            .is_some_and(|latest_epoch| request.epoch < latest_epoch)
        {
            return Err(ApexError::State(
                "flow request epoch cannot move backwards".to_owned(),
            ));
        }
        if self.mode == ControlMode::Halted && request.kind != FlowKind::Penalty {
            return Err(ApexError::State("economic outflows are halted".to_owned()));
        }
        let max_single = if self.mode == ControlMode::Restricted {
            self.limits.restricted_max_single
        } else {
            self.limits.max_single
        };
        if request.amount > max_single {
            return Err(ApexError::State(
                "single-operation flow limit exceeded".to_owned(),
            ));
        }
        if request.amount >= self.limits.large_operation_threshold {
            let approvals: BTreeSet<_> = request
                .approvers
                .iter()
                .copied()
                .filter(|account| self.limits.authorized_approvers.contains(account))
                .collect();
            if approvals.len() < self.limits.large_operation_approvals {
                return Err(ApexError::Unauthorized(
                    "large-operation approval threshold not met".to_owned(),
                ));
            }
        }
        let window_start = request.epoch.saturating_sub(self.limits.window_epochs);
        let mut window_total = Amount::zero();
        for record in self.records.values() {
            if record.request.epoch > window_start && record.request.epoch <= request.epoch {
                window_total = window_total.checked_add(record.request.amount)?;
            }
        }
        if window_total.checked_add(request.amount)? > self.limits.max_window {
            return Err(ApexError::State("rolling flow limit exceeded".to_owned()));
        }
        let record = FlowRecord {
            request: request.clone(),
            mode: self.mode,
        };
        self.latest_epoch = Some(request.epoch);
        self.records.insert(request.id, record.clone());
        Ok(record)
    }

    pub fn records(&self) -> impl Iterator<Item = &FlowRecord> {
        self.records.values()
    }
}
