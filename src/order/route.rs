use serde::Serialize;

use crate::{AccountId, Amount, ApexError, ApexResult, Digest, IntentPolicy};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LiquidityCredit {
    pub provider: AccountId,
    pub amount: Amount,
    pub reference: Digest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoutePlan {
    pub venue: Digest,
    pub solver: AccountId,
    pub fee_recipient: AccountId,
    pub operator_fee: Amount,
    pub rebate_recipient: Option<AccountId>,
    pub rebate_amount: Amount,
    pub credit: LiquidityCredit,
    pub execution_lane: u16,
    pub quote_nonce: u64,
}

impl LiquidityCredit {
    pub fn none(provider: AccountId) -> Self {
        Self {
            provider,
            amount: Amount::zero(),
            reference: Digest::from_parts("apex-empty-credit-v1", &[&provider.bytes()]),
        }
    }
}

impl RoutePlan {
    pub fn direct(solver: AccountId, venue: Digest, lane: u16, nonce: u64) -> Self {
        Self {
            venue,
            solver,
            fee_recipient: solver,
            operator_fee: Amount::zero(),
            rebate_recipient: None,
            rebate_amount: Amount::zero(),
            credit: LiquidityCredit::none(solver),
            execution_lane: lane,
            quote_nonce: nonce,
        }
    }

    pub fn gross_charges(self) -> ApexResult<Amount> {
        self.operator_fee.checked_add(self.rebate_amount)
    }

    pub fn policy_charges(self) -> ApexResult<Amount> {
        Ok(self.gross_charges()?.checked_sub_floor(self.credit.amount))
    }

    pub fn route_digest(self) -> ApexResult<Digest> {
        Digest::from_serializable("apex-route-plan-v1", &self)
    }

    pub fn validate(self, policy: IntentPolicy, amount: Amount) -> ApexResult<()> {
        if self.venue != policy.venue {
            return Err(ApexError::Policy("execution venue mismatch".to_owned()));
        }

        if self.execution_lane != policy.execution_lane {
            return Err(ApexError::Policy("execution lane mismatch".to_owned()));
        }

        let gross = self.gross_charges()?;
        if gross > amount {
            return Err(ApexError::Policy(
                "route charges exceed locked amount".to_owned(),
            ));
        }

        let allowed = amount.checked_mul_bps(policy.max_charge_bps)?;
        let observed = self.policy_charges()?;
        if observed > allowed {
            return Err(ApexError::Policy(
                "route charges exceed policy ceiling".to_owned(),
            ));
        }

        Ok(())
    }
}
