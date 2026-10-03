use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyndicateSettings {
    pub wts: LiveSyndicateWtsSettings,
}
