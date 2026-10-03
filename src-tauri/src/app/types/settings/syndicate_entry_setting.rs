use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyndicateEntrySetting {
    pub name: String,
    pub unique_name: String,
    pub standing: i64,
    pub ignore_standing: bool,
}

impl SyndicateEntrySetting {
    pub fn can_post(&self, cost: i64) -> bool {
        if self.ignore_standing {
            return true;
        }
        if self.standing >= cost {
            true
        } else {
            false
        }
    }
}
