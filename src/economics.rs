use serde::Serialize;

use crate::{Amount, ApexError, ApexResult, Bps};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FeeCurve {
    pub base_fee_bps: Bps,
    pub liquidity_cost_bps: Bps,
    pub utilization_kink_bps: Bps,
    pub slope_below_kink_bps: u16,
    pub slope_above_kink_bps: u16,
    pub finality_premium_per_epoch_bps: u16,
    pub max_fee_bps: Bps,
}

impl Default for FeeCurve {
    fn default() -> Self {
        Self {
            base_fee_bps: Bps::new(8).expect("constant fee must be valid"),
            liquidity_cost_bps: Bps::new(4).expect("constant fee must be valid"),
            utilization_kink_bps: Bps::new(8_000).expect("constant kink must be valid"),
            slope_below_kink_bps: 12,
            slope_above_kink_bps: 80,
            finality_premium_per_epoch_bps: 1,
            max_fee_bps: Bps::new(250).expect("constant fee must be valid"),
        }
    }
}

impl FeeCurve {
    pub fn validate(self) -> ApexResult<()> {
        let kink = self.utilization_kink_bps.units();
        if kink == 0 || kink == 10_000 {
            return Err(ApexError::InvalidConfiguration(
                "utilization kink must be inside the open interval".to_owned(),
            ));
        }
        if self.base_fee_bps > self.max_fee_bps {
            return Err(ApexError::InvalidConfiguration(
                "base fee exceeds maximum fee".to_owned(),
            ));
        }
        Ok(())
    }

    fn utilization_premium(self, utilization: Bps) -> u32 {
        let utilization = u32::from(utilization.units());
        let kink = u32::from(self.utilization_kink_bps.units());
        if utilization <= kink {
            return utilization * u32::from(self.slope_below_kink_bps) / kink;
        }
        let above = (utilization - kink) * u32::from(self.slope_above_kink_bps) / (10_000 - kink);
        u32::from(self.slope_below_kink_bps) + above
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EconomicInputs {
    pub principal: Amount,
    pub utilization_bps: Bps,
    pub finality_epochs: u64,
    pub loss_probability_ppm: u32,
    pub loss_severity_bps: Bps,
    pub risk_score_bps: Bps,
}

impl EconomicInputs {
    pub fn validate(self) -> ApexResult<()> {
        if self.principal.is_zero() {
            return Err(ApexError::ZeroAmount);
        }
        if self.loss_probability_ppm > 1_000_000 {
            return Err(ApexError::InvalidConfiguration(
                "loss probability exceeds one million ppm".to_owned(),
            ));
        }
        if self.finality_epochs > 1_000 {
            return Err(ApexError::InvalidConfiguration(
                "finality horizon exceeds operational maximum".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EconomicQuote {
    pub quote_digest: crate::Digest,
    pub effective_fee_bps: Bps,
    pub utilization_premium_bps: u32,
    pub finality_premium_bps: u32,
    pub risk_premium_bps: u32,
    pub protocol_fee: Amount,
    pub expected_loss: Amount,
    pub liquidity_cost: Amount,
    pub risk_reserve: Amount,
    pub contribution_surplus: Amount,
    pub contribution_shortfall: Amount,
}

impl EconomicQuote {
    pub fn sustainable(&self) -> bool {
        self.contribution_shortfall.is_zero()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PricingEngine {
    pub curve: FeeCurve,
}

impl PricingEngine {
    pub fn new(curve: FeeCurve) -> ApexResult<Self> {
        curve.validate()?;
        Ok(Self { curve })
    }

    pub fn quote(self, inputs: EconomicInputs) -> ApexResult<EconomicQuote> {
        self.curve.validate()?;
        inputs.validate()?;
        let utilization_premium = self.curve.utilization_premium(inputs.utilization_bps);
        let finality_premium = inputs
            .finality_epochs
            .checked_mul(u64::from(self.curve.finality_premium_per_epoch_bps))
            .ok_or(ApexError::AmountOverflow)?;
        let risk_premium = u32::from(inputs.risk_score_bps.units()) / 200;
        let uncapped = u32::from(self.curve.base_fee_bps.units())
            .checked_add(utilization_premium)
            .and_then(|value| value.checked_add(u32::try_from(finality_premium).ok()?))
            .and_then(|value| value.checked_add(risk_premium))
            .ok_or(ApexError::AmountOverflow)?;
        let effective = uncapped.min(u32::from(self.curve.max_fee_bps.units()));
        let effective_fee_bps = Bps::new(u16::try_from(effective).map_err(|_| {
            ApexError::InvalidConfiguration("effective fee exceeds basis-point scale".to_owned())
        })?)?;
        let protocol_fee = inputs.principal.checked_mul_bps(effective_fee_bps)?;
        let expected_loss = inputs
            .principal
            .checked_mul_ppm(inputs.loss_probability_ppm)?
            .checked_mul_bps(inputs.loss_severity_bps)?;
        let liquidity_cost = inputs
            .principal
            .checked_mul_bps(self.curve.liquidity_cost_bps)?;
        let risk_reserve = expected_loss.checked_mul_ratio(12_500, 10_000)?;
        let costs = expected_loss.checked_add(liquidity_cost)?;
        let (contribution_surplus, contribution_shortfall) = if protocol_fee >= costs {
            (protocol_fee.checked_sub(costs)?, Amount::zero())
        } else {
            (Amount::zero(), costs.checked_sub(protocol_fee)?)
        };
        let quote_digest = crate::Digest::from_serializable(
            "apex-economic-quote-v1",
            &(
                inputs,
                effective_fee_bps,
                protocol_fee,
                expected_loss,
                liquidity_cost,
                risk_reserve,
            ),
        )?;
        Ok(EconomicQuote {
            quote_digest,
            effective_fee_bps,
            utilization_premium_bps: utilization_premium,
            finality_premium_bps: u32::try_from(finality_premium)
                .map_err(|_| ApexError::AmountOverflow)?,
            risk_premium_bps: risk_premium,
            protocol_fee,
            expected_loss,
            liquidity_cost,
            risk_reserve,
            contribution_surplus,
            contribution_shortfall,
        })
    }
}
