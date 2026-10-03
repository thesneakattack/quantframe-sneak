use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RivenSettings {
    pub general: RivenGeneralSettings,
    pub wts: RivenWtsSettings,
}
