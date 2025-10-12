// src/domain/value_objects/api_key_id.rs

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiKeyId(Uuid);

impl ApiKeyId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for ApiKeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ApiKeyId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ApiKeyId> for Uuid {
    fn from(api_key_id: ApiKeyId) -> Self {
        api_key_id.0
    }
}

impl Default for ApiKeyId {
    fn default() -> Self {
        Self::new()
    }
}
