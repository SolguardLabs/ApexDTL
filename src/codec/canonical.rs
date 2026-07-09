use serde::Serialize;

use crate::{ApexError, ApexResult};

pub fn canonical_bytes<T: Serialize>(value: &T) -> ApexResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| ApexError::Serialization(error.to_string()))
}
