use apex_dtl::{
    Amount, ApexError, ApexResult, Bps, CorridorLimits, EconomicInputs, FeeCurve,
    PortfolioExposure, PricingEngine, RiskBand, RiskEngine, RiskSignals, RiskWeights,
};

#[test]
fn risk_engine_applies_concentration_and_collateral_band() -> ApexResult<()> {
    let engine = RiskEngine::new(RiskWeights::default())?;
    let decision = engine.assess(
        Amount::new(500_000)?,
        PortfolioExposure {
            corridor_open: Amount::new(500_000)?,
            portfolio_open: Amount::new(2_000_000)?,
        },
        RiskSignals {
            finality_bps: 2_000,
            liquidity_bps: 3_000,
            counterparty_bps: 5_000,
            operational_bps: 4_000,
        },
        CorridorLimits {
            max_principal: Amount::new(2_000_000)?,
            max_concentration_bps: Bps::new(6_000)?,
            min_collateral_bps: 12_000,
        },
    )?;
    assert!(decision.accepted);
    assert_eq!(decision.risk_score_bps.units(), 3_650);
    assert_eq!(decision.band, RiskBand::Standard);
    assert_eq!(decision.concentration_bps.units(), 4_000);
    assert_eq!(decision.effective_collateral_bps, 12_000);
    assert_eq!(decision.required_collateral.units(), 600_000);
    Ok(())
}

#[test]
fn risk_engine_rejects_restricted_corridor_concentration() -> ApexResult<()> {
    let engine = RiskEngine::new(RiskWeights::default())?;
    let decision = engine.assess(
        Amount::new(400_000)?,
        PortfolioExposure {
            corridor_open: Amount::new(900_000)?,
            portfolio_open: Amount::new(2_000_000)?,
        },
        RiskSignals {
            finality_bps: 8_000,
            liquidity_bps: 7_000,
            counterparty_bps: 8_500,
            operational_bps: 7_500,
        },
        CorridorLimits {
            max_principal: Amount::new(1_000_000)?,
            max_concentration_bps: Bps::new(3_000)?,
            min_collateral_bps: 15_000,
        },
    )?;
    assert!(!decision.accepted);
    assert_eq!(decision.band, RiskBand::Restricted);
    assert_eq!(decision.required_collateral.units(), 720_000);
    assert_eq!(decision.reasons.len(), 1);
    Ok(())
}

#[test]
fn pricing_engine_decomposes_risk_adjusted_margin() -> ApexResult<()> {
    let quote = PricingEngine::new(FeeCurve::default())?.quote(EconomicInputs {
        principal: Amount::new(10_000_000)?,
        utilization_bps: Bps::new(9_000)?,
        finality_epochs: 6,
        loss_probability_ppm: 2_000,
        loss_severity_bps: Bps::new(4_000)?,
        risk_score_bps: Bps::new(4_000)?,
    })?;
    assert_eq!(quote.utilization_premium_bps, 52);
    assert_eq!(quote.finality_premium_bps, 6);
    assert_eq!(quote.risk_premium_bps, 20);
    assert_eq!(quote.effective_fee_bps.units(), 86);
    assert_eq!(quote.protocol_fee.units(), 86_000);
    assert_eq!(quote.expected_loss.units(), 8_000);
    assert_eq!(quote.liquidity_cost.units(), 4_000);
    assert_eq!(quote.risk_reserve.units(), 10_000);
    assert_eq!(quote.contribution_surplus.units(), 74_000);
    assert!(quote.sustainable());
    Ok(())
}

#[test]
fn pricing_engine_caps_fee_and_reports_shortfall() -> ApexResult<()> {
    let capped = PricingEngine::new(FeeCurve::default())?.quote(EconomicInputs {
        principal: Amount::new(1_000_000)?,
        utilization_bps: Bps::new(10_000)?,
        finality_epochs: 1_000,
        loss_probability_ppm: 0,
        loss_severity_bps: Bps::new(0)?,
        risk_score_bps: Bps::new(10_000)?,
    })?;
    assert_eq!(capped.effective_fee_bps.units(), 250);

    let curve = FeeCurve {
        base_fee_bps: Bps::new(5)?,
        liquidity_cost_bps: Bps::new(100)?,
        utilization_kink_bps: Bps::new(8_000)?,
        slope_below_kink_bps: 0,
        slope_above_kink_bps: 0,
        finality_premium_per_epoch_bps: 0,
        max_fee_bps: Bps::new(5)?,
    };
    let shortfall = PricingEngine::new(curve)?.quote(EconomicInputs {
        principal: Amount::new(1_000_000)?,
        utilization_bps: Bps::new(0)?,
        finality_epochs: 0,
        loss_probability_ppm: 0,
        loss_severity_bps: Bps::new(0)?,
        risk_score_bps: Bps::new(0)?,
    })?;
    assert_eq!(shortfall.protocol_fee.units(), 500);
    assert_eq!(shortfall.liquidity_cost.units(), 10_000);
    assert_eq!(shortfall.contribution_shortfall.units(), 9_500);
    assert!(!shortfall.sustainable());
    Ok(())
}

#[test]
fn amount_ratio_and_risk_weights_fail_closed() -> ApexResult<()> {
    assert!(matches!(
        Amount::new(1)?.checked_mul_ratio(1, 0),
        Err(ApexError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        RiskEngine::new(RiskWeights {
            finality: 2_500,
            liquidity: 2_500,
            counterparty: 2_500,
            operational: 2_499,
        }),
        Err(ApexError::InvalidConfiguration(_))
    ));
    Ok(())
}
