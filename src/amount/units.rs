use serde::{Deserialize, Serialize};

use crate::{ApexError, ApexResult};

#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Amount(u128);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bps(u16);

impl Amount {
    pub const fn zero() -> Self {
        Self(0)
    }

    pub fn new(units: u128) -> ApexResult<Self> {
        Ok(Self(units))
    }

    pub const fn units(self) -> u128 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn checked_add(self, rhs: Self) -> ApexResult<Self> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(ApexError::AmountOverflow)
    }

    pub fn checked_sub(self, rhs: Self) -> ApexResult<Self> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(ApexError::AmountUnderflow)
    }

    pub fn checked_sub_floor(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn checked_mul_bps(self, bps: Bps) -> ApexResult<Self> {
        self.0
            .checked_mul(u128::from(bps.units()))
            .and_then(|value| value.checked_div(10_000))
            .map(Self)
            .ok_or(ApexError::AmountOverflow)
    }
}

impl Bps {
    pub fn new(units: u16) -> ApexResult<Self> {
        if units > 10_000 {
            return Err(ApexError::BpsOutOfRange(units));
        }
        Ok(Self(units))
    }

    pub const fn units(self) -> u16 {
        self.0
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::fmt::Display for Bps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
