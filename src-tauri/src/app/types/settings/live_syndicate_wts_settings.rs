use super::*;
use serde::{Deserialize, Serialize};
use utils::{get_location, Error};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveSyndicateWtsSettings {
    pub syndicates: Vec<SyndicateEntrySetting>,
    pub volume_threshold: i64,
    pub min_price: i64,
    pub max_price_drop: i64,
    pub min_listings_below: i64,
}

impl LiveSyndicateWtsSettings {
    pub fn find_syndicate(&self, syndicate_id: impl AsRef<str>) -> Option<&SyndicateEntrySetting> {
        let syndicate_id = syndicate_id.as_ref();

        self.syndicates
            .iter()
            .find(|entry| entry.unique_name == syndicate_id)
    }

    pub fn find_syndicate_mut(
        &mut self,
        syndicate_id: impl AsRef<str>,
    ) -> Option<&mut SyndicateEntrySetting> {
        let syndicate_id = syndicate_id.as_ref();

        self.syndicates
            .iter_mut()
            .find(|entry| entry.unique_name == syndicate_id)
    }

    pub fn can_afford_posting(
        &self,
        syndicate_id: impl AsRef<str>,
        cost: i64,
    ) -> Result<bool, Error> {
        let syndicate_id = syndicate_id.as_ref();

        self.find_syndicate(syndicate_id)
            .map(|entry| entry.can_post(cost))
            .ok_or_else(|| syndicate_not_found(syndicate_id))
    }

    pub fn deduct_standing(
        &mut self,
        syndicate_id: impl AsRef<str>,
        amount: i64,
    ) -> Result<(), Error> {
        let syndicate_id = syndicate_id.as_ref();

        let entry = self
            .find_syndicate_mut(syndicate_id)
            .ok_or_else(|| syndicate_not_found(syndicate_id))?;
        if !entry.ignore_standing {
            entry.standing -= amount;
        }

        Ok(())
    }

    pub fn get_syndicate_ids(&self) -> Vec<String> {
        self.syndicates
            .iter()
            .map(|entry| entry.unique_name.clone())
            .collect()
    }
}

impl Default for LiveSyndicateWtsSettings {
    fn default() -> Self {
        Self {
            syndicates: Vec::new(),
            volume_threshold: 10,
            min_price: 10,
            max_price_drop: -1,
            min_listings_below: -1,
        }
    }
}

/// --------------------
/// HELPER METHODS
/// --------------------
fn syndicate_not_found(syndicate_id: &str) -> Error {
    Error::new(
        "LiveSyndicateWtsSettings",
        &format!("Syndicate with ID {syndicate_id} not found"),
        get_location!(),
    )
}
