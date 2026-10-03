use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[derive(Default)]
pub struct RivenSettings {
    pub general: RivenGeneralSettings,
    pub wts: RivenWtsSettings,
}

