use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AdvancedSettings {
    pub http_server: HttpServerSettings,
}
