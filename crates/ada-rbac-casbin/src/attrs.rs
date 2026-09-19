use std::net::IpAddr as StdIpAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attrs {
    pub tenant_id: String,
    pub is_owner: bool,
    pub now_utc: DateTime<Utc>,
    pub request_ip: Option<IpAddr>,
}

impl Attrs {
    #[must_use]
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            is_owner: false,
            now_utc: Utc::now(),
            request_ip: None,
        }
    }

    #[must_use]
    pub fn with_owner_flag(mut self, is_owner: bool) -> Self {
        self.is_owner = is_owner;
        self
    }

    #[must_use]
    pub fn with_now(mut self, now_utc: DateTime<Utc>) -> Self {
        self.now_utc = now_utc;
        self
    }

    #[must_use]
    pub fn with_request_ip(mut self, ip: StdIpAddr) -> Self {
        self.request_ip = Some(IpAddr::from(ip));
        self
    }
}