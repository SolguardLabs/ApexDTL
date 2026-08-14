use serde::Serialize;

use crate::{Amount, ApexError, ApexResult, Bps};

const BPS_SCALE: u128 = 10_000;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RiskSignals {
    pub finality_bps: u16,
    pub liquidity_bps: u16,
    pub counterparty_bps: u16,
    pub operational_bps: u16,
}

impl RiskSignals {
    pub fn validate(self) -> ApexResult<()> {
        for value in [
            self.finality_bps,
            self.liquidity_bps,
            self.counterparty_bps,
            self.operational_bps,
        ] {
            Bps::new(value)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RiskWeights {
    pub finality: u16,
    pub liquidity: u16,
    pub counterparty: u16,
    pub operational: u16,
}

impl Default for RiskWeights {
    fn default() -> Self {
        Self {
            finality: 2_500,
            liquidity: 2_000,
            counterparty: 3_500,
            operational: 2_000,
        }
    }
}

impl RiskWeights {
    pub fn validate(self) -> ApexResult<()> {
        let values = [
            self.finality,
            self.liquidity,
            self.counterparty,
            self.operational,
        ];
        for value in values {
            Bps::new(value)?;
        }
        let total: u32 = values.into_iter().map(u32::from).sum();
        if total != 10_000 {
            return Err(ApexError::InvalidConfiguration(
                "risk weights must sum to 10000".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn score(self, signals: RiskSignals) -> ApexResult<Bps> {
        self.validate()?;
        signals.validate()?;
        let weighted = u64::from(signals.finality_bps) * u64::from(self.finality)
            + u64::from(signals.liquidity_bps) * u64::from(self.liquidity)
            + u64::from(signals.counterparty_bps) * u64::from(self.counterparty)
            + u64::from(signals.operational_bps) * u64::from(self.operational);
        Bps::new((weighted / 10_000) as u16)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskBand {
    Core,
    Standard,
    Elevated,
    Restricted,
}

impl RiskBand {
    pub fn from_score(score: Bps) -> Self {
        match score.units() {
            0..=1_999 => Self::Core,
            2_000..=4_499 => Self::Standard,
            4_500..=6_999 => Self::Elevated,
            _ => Self::Restricted,
        }
    }

    pub const fn collateral_bps(self) -> u32 {
        match self {
            Self::Core => 10_000,
            Self::Standard => 11_500,
            Self::Elevated => 14_000,
            Self::Restricted => 18_000,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CorridorLimits {
    pub max_principal: Amount,
    pub max_concentration_bps: Bps,
    pub min_collateral_bps: u32,
}

impl CorridorLimits {
    pub fn validate(self) -> ApexResult<()> {
        if self.max_principal.is_zero() {
            return Err(ApexError::InvalidConfiguration(
                "corridor max principal must be positive".to_owned(),
            ));
        }
        if self.max_concentration_bps.units() == 0 {
            return Err(ApexError::InvalidConfiguration(
                "corridor concentration limit must be positive".to_owned(),
            ));
        }
        if !(10_000..=50_000).contains(&self.min_collateral_bps) {
            return Err(ApexError::InvalidConfiguration(
                "collateral ratio is outside the supported range".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PortfolioExposure {
    pub corridor_open: Amount,
    pub portfolio_open: Amount,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdmissionDecision {
    pub accepted: bool,
    pub risk_score_bps: Bps,
    pub band: RiskBand,
    pub concentration_bps: Bps,
    pub effective_collateral_bps: u32,
    pub required_collateral: Amount,
    pub reasons: Vec<String>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RiskEngine {
    pub weights: RiskWeights,
}

impl RiskEngine {
    pub fn new(weights: RiskWeights) -> ApexResult<Self> {
        weights.validate()?;
        Ok(Self { weights })
    }

    pub fn assess(
        self,
        principal: Amount,
        exposure: PortfolioExposure,
        signals: RiskSignals,
        limits: CorridorLimits,
    ) -> ApexResult<AdmissionDecision> {
        limits.validate()?;
        if principal.is_zero() {
            return Err(ApexError::ZeroAmount);
        }
        if exposure.corridor_open > exposure.portfolio_open {
            return Err(ApexError::InvalidConfiguration(
                "corridor exposure exceeds portfolio exposure".to_owned(),
            ));
        }
        let score = self.weights.score(signals)?;
        let band = RiskBand::from_score(score);
        let next_corridor = exposure.corridor_open.checked_add(principal)?;
        let next_portfolio = exposure.portfolio_open.checked_add(principal)?;
        let concentration = next_corridor
            .checked_mul_ratio(BPS_SCALE, next_portfolio.units())?
            .units();
        let concentration_bps = Bps::new(u16::try_from(concentration).map_err(|_| {
            ApexError::InvalidConfiguration("concentration does not fit basis points".to_owned())
        })?)?;
        let effective_collateral_bps = limits.min_collateral_bps.max(band.collateral_bps());
        let required_collateral =
            principal.checked_mul_ratio(u128::from(effective_collateral_bps), BPS_SCALE)?;
        let mut reasons = Vec::with_capacity(2);
        if principal > limits.max_principal {
            reasons.push("principal exceeds corridor limit".to_owned());
        }
        if concentration_bps > limits.max_concentration_bps {
            reasons.push("post-trade concentration exceeds corridor limit".to_owned());
        }
        Ok(AdmissionDecision {
            accepted: reasons.is_empty(),
            risk_score_bps: score,
            band,
            concentration_bps,
            effective_collateral_bps,
            required_collateral,
            reasons,
        })
    }
}
