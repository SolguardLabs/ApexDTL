use serde::Serialize;

use crate::{
    AccountId, Amount, ApexError, ApexResult, AssetId, Bps, Digest, IntentId, KeyPair,
    PublicIdentity, RoutePlan, SignatureBytes, TxId, verify_signature,
};

pub const INTENT_DOMAIN: &str = "apex-intent-open-v1";

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntentPolicy {
    pub venue: Digest,
    pub max_charge_bps: Bps,
    pub execution_lane: u16,
    pub valid_after_epoch: u64,
    pub expires_at_epoch: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntentTerms {
    pub network_id: u32,
    pub intent_id: IntentId,
    pub payer: AccountId,
    pub beneficiary: AccountId,
    pub asset: AssetId,
    pub amount: Amount,
    pub payer_nonce: u64,
    pub policy: IntentPolicy,
    pub salt: Digest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntentAuthorizationView {
    network_id: u32,
    intent_id: IntentId,
    payer: AccountId,
    beneficiary: AccountId,
    asset: AssetId,
    amount: Amount,
    payer_nonce: u64,
    policy: IntentPolicy,
    salt: Digest,
    route_digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignedIntent {
    pub payer: PublicIdentity,
    pub terms: IntentTerms,
    pub route: RoutePlan,
    pub signature: SignatureBytes,
}

impl IntentPolicy {
    pub fn new(
        venue: Digest,
        max_charge_bps: Bps,
        execution_lane: u16,
        valid_after_epoch: u64,
        expires_at_epoch: u64,
    ) -> Self {
        Self {
            venue,
            max_charge_bps,
            execution_lane,
            valid_after_epoch,
            expires_at_epoch,
        }
    }
}

impl IntentTerms {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        network_id: u32,
        payer: AccountId,
        beneficiary: AccountId,
        asset: AssetId,
        amount: Amount,
        payer_nonce: u64,
        policy: IntentPolicy,
        salt: Digest,
    ) -> ApexResult<Self> {
        if amount.is_zero() {
            return Err(ApexError::ZeroAmount);
        }
        let intent_id = IntentId::derive(network_id, payer, beneficiary, payer_nonce, salt);
        Ok(Self {
            network_id,
            intent_id,
            payer,
            beneficiary,
            asset,
            amount,
            payer_nonce,
            policy,
            salt,
        })
    }

    pub fn authorization_view(self, route: RoutePlan) -> ApexResult<IntentAuthorizationView> {
        Ok(IntentAuthorizationView {
            network_id: self.network_id,
            intent_id: self.intent_id,
            payer: self.payer,
            beneficiary: self.beneficiary,
            asset: self.asset,
            amount: self.amount,
            payer_nonce: self.payer_nonce,
            policy: self.policy,
            salt: self.salt,
            route_digest: route.route_digest()?,
        })
    }
}

impl SignedIntent {
    pub fn sign(terms: IntentTerms, route: RoutePlan, key_pair: &KeyPair) -> ApexResult<Self> {
        let payer = key_pair.public_identity();
        if payer.account != terms.payer {
            return Err(ApexError::UnauthorizedIntentSigner {
                expected: terms.payer,
                received: payer.account,
            });
        }
        let signature = key_pair.sign(INTENT_DOMAIN, &terms.authorization_view(route)?)?;
        Ok(Self {
            payer,
            terms,
            route,
            signature,
        })
    }

    pub fn verify(&self) -> ApexResult<()> {
        if self.payer.account != self.terms.payer {
            return Err(ApexError::UnauthorizedIntentSigner {
                expected: self.terms.payer,
                received: self.payer.account,
            });
        }
        verify_signature(
            self.payer,
            self.signature,
            INTENT_DOMAIN,
            &self.terms.authorization_view(self.route)?,
        )
    }

    pub fn tx_id(&self) -> ApexResult<TxId> {
        TxId::from_serializable("apex-signed-intent-v1", self)
    }
}
