use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::cache::{modules::LanguageModule, *};
use utils::{get_location, info, read_json_file_optional, Error, LoggerOptions, MultiKeyMap};

#[derive(Debug)]
pub struct SyndicateModule {
    path: PathBuf,

    syndicate_lookup: Mutex<MultiKeyMap<CacheSyndicate>>,
    favour_lookup: Mutex<MultiKeyMap<CacheSyndicateFavour>>,
}

impl SyndicateModule {
    pub fn new(client: Arc<CacheState>) -> Arc<Self> {
        Arc::new(Self {
            path: client.base_path.join("items/Syndicate.json"),
            syndicate_lookup: Mutex::new(MultiKeyMap::new()),
            favour_lookup: Mutex::new(MultiKeyMap::new()),
        })
    }

    pub fn load(&self, _language: &LanguageModule) -> Result<(), Error> {
        let items = read_json_file_optional::<Vec<CacheSyndicate>>(&self.path)
            .map_err(|e| e.with_location(get_location!()))?;

        let mut syndicate_lookup = self.syndicate_lookup.lock().unwrap();
        let mut favour_lookup = self.favour_lookup.lock().unwrap();

        for syndicate in items {
            let mut syndicate_keys = vec![syndicate.name.clone(), syndicate.unique_name.clone()];

            // Also allow lookup by the last part of the unique name.
            if let Some(key) = syndicate.unique_name.rsplit('/').next() {
                if !syndicate_keys.iter().any(|existing| existing == key) {
                    syndicate_keys.push(key.to_owned());
                }
            }

            syndicate_lookup.insert_value(syndicate.clone(), syndicate_keys.clone());

            for favour in &syndicate.favours {
                if favour.unique_name.is_empty() {
                    continue;
                }
                let mut favour_keys = vec![favour.unique_name.clone()];

                for syndicate_key in &syndicate_keys {
                    favour_keys.push(format!("{}|{}", syndicate_key, favour.unique_name));
                }

                favour_lookup.insert_value(favour.clone(), favour_keys);
            }
        }

        info(
            "Cache:Syndicate:load",
            format!(
                "Loaded {} syndicates and {} favours from cache",
                syndicate_lookup.len(),
                favour_lookup.len()
            ),
            &LoggerOptions::default(),
        );

        Ok(())
    }

    // -------------------------------------------------------------
    // Lookup functions
    // -------------------------------------------------------------

    /// Gets a syndicate by name, unique name, or unique-name suffix.
    pub fn get_by(&self, syndicate_id: impl Into<String>) -> Result<CacheSyndicate, Error> {
        let syndicate_id = syndicate_id.into();
        let lookup = self.syndicate_lookup.lock().unwrap();

        lookup.get(&syndicate_id).cloned().ok_or_else(|| {
            Error::new(
                "Cache:Syndicate:GetBy",
                format!("Syndicate not found for id '{}'", syndicate_id),
                get_location!(),
            )
        })
    }

    /// Gets a favour by syndicate identifier and favour identifier.
    pub fn get_favour_by(
        &self,
        syndicate_id: impl Into<String>,
        favour_id: impl Into<String>,
    ) -> Result<CacheSyndicateFavour, Error> {
        let key = format!("{}|{}", syndicate_id.into(), favour_id.into());
        let lookup = self.favour_lookup.lock().unwrap();

        lookup.get(&key).cloned().ok_or_else(|| {
            Error::new(
                "Cache:Syndicate:GetFavourBy",
                format!("Favour not found for key '{}'", key),
                get_location!(),
            )
        })
    }

    // -------------------------------------------------------------
    // Vector functions
    // -------------------------------------------------------------

    pub fn get_items(&self) -> Result<Vec<CacheSyndicate>, Error> {
        let lookup = self.syndicate_lookup.lock().unwrap();
        Ok(lookup.get_all_values())
    }
}
