use serde::Serialize;

use crate::{
    AccountId, ApexError, ApexResult, Digest, IntentId, KeyPair, PublicIdentity, SignatureBytes,
    TxId, verify_signature,
};

pub const SETTLEMENT_DOMAIN: &str = "apex-intent-settlement-v1";

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SettlementRequest {
    pub network_id: u32,
    pub intent_id: IntentId,
    pub beneficiary: AccountId,
    pub settlement_nonce: u64,
    pub observed_route_digest: Digest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SettlementAuthorizationView {
    network_id: u32,
    intent_id: IntentId,
    beneficiary: AccountId,
    settlement_nonce: u64,
    observed_route_digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignedSettlement {
    pub signer: PublicIdentity,
    pub request: SettlementRequest,
    pub signature: SignatureBytes,
}

impl SettlementRequest {
    pub fn new(
        network_id: u32,
        intent_id: IntentId,
        beneficiary: AccountId,
        settlement_nonce: u64,
        observed_route_digest: Digest,
    ) -> Self {
        Self {
            network_id,
            intent_id,
            beneficiary,
            settlement_nonce,
            observed_route_digest,
        }
    }

    pub fn authorization_view(self) -> SettlementAuthorizationView {
        SettlementAuthorizationView {
            network_id: self.network_id,
            intent_id: self.intent_id,
            beneficiary: self.beneficiary,
            settlement_nonce: self.settlement_nonce,
            observed_route_digest: self.observed_route_digest,
        }
    }
}

impl SignedSettlement {
    pub fn sign(request: SettlementRequest, key_pair: &KeyPair) -> ApexResult<Self> {
        let signer = key_pair.public_identity();
        if signer.account != request.beneficiary {
            return Err(ApexError::UnauthorizedSettlementSigner {
                expected: request.beneficiary,
                received: signer.account,
            });
        }
        let signature = key_pair.sign(SETTLEMENT_DOMAIN, &request.authorization_view())?;
        Ok(Self {
            signer,
            request,
            signature,
        })
    }

    pub fn verify(&self) -> ApexResult<()> {
        if self.signer.account != self.request.beneficiary {
            return Err(ApexError::UnauthorizedSettlementSigner {
                expected: self.request.beneficiary,
                received: self.signer.account,
            });
        }
        verify_signature(
            self.signer,
            self.signature,
            SETTLEMENT_DOMAIN,
            &self.request.authorization_view(),
        )
    }

    pub fn tx_id(&self) -> ApexResult<TxId> {
        TxId::from_serializable("apex-signed-settlement-v1", self)
    }
}
