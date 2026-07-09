use serde::{Serialize, Serializer};

use crate::{ApexResult, canonical_bytes};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AccountId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AssetId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IntentId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TxId([u8; 32]);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Digest([u8; 32]);

macro_rules! id_type {
    ($name:ident) => {
        impl $name {
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub const fn bytes(self) -> [u8; 32] {
                self.0
            }

            pub fn from_serializable<T: Serialize>(domain: &str, value: &T) -> ApexResult<Self> {
                let digest = Digest::from_serializable(domain, value)?;
                Ok(Self(digest.bytes()))
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&hex::encode(self.0))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", hex::encode(self.0))
            }
        }
    };
}

id_type!(AccountId);
id_type!(AssetId);
id_type!(IntentId);
id_type!(TxId);

impl Digest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl Serialize for Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl std::fmt::Display for Digest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", hex::encode(self.0))
    }
}

impl AssetId {
    pub fn native() -> Self {
        Digest::from_parts("apex-native-asset-v1", &[b"APEX"]).into()
    }
}

impl IntentId {
    pub fn derive(
        network_id: u32,
        payer: AccountId,
        beneficiary: AccountId,
        nonce: u64,
        salt: Digest,
    ) -> Self {
        Digest::from_parts(
            "apex-intent-id-v1",
            &[
                &network_id.to_be_bytes(),
                &payer.bytes(),
                &beneficiary.bytes(),
                &nonce.to_be_bytes(),
                &salt.bytes(),
            ],
        )
        .into()
    }
}

impl Digest {
    pub fn from_parts(domain: &str, parts: &[&[u8]]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(domain.as_bytes());
        for part in parts {
            hasher.update(&(part.len() as u64).to_be_bytes());
            hasher.update(part);
        }
        Self(*hasher.finalize().as_bytes())
    }

    pub fn from_serializable<T: Serialize>(domain: &str, value: &T) -> ApexResult<Self> {
        let bytes = canonical_bytes(value)?;
        Ok(Self::from_parts(domain, &[&bytes]))
    }
}

impl From<Digest> for AccountId {
    fn from(value: Digest) -> Self {
        Self(value.bytes())
    }
}

impl From<Digest> for AssetId {
    fn from(value: Digest) -> Self {
        Self(value.bytes())
    }
}

impl From<Digest> for IntentId {
    fn from(value: Digest) -> Self {
        Self(value.bytes())
    }
}

impl From<Digest> for TxId {
    fn from(value: Digest) -> Self {
        Self(value.bytes())
    }
}
