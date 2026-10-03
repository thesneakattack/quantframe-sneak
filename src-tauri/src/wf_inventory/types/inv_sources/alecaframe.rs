use crate::{utils::modules::states, wf_inventory::WarframeRootObject};
use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit};
type DecryptThingy = cbc::Decryptor<aes::Aes128>;
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use utils::*;

use serde::{Deserialize, Serialize};

use super::{helpers::*, traits::InventorySource};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WFInvAlecaframeSource {
    pub path: String,
    #[serde(skip, default = "default_stop_flag")]
    stop_flag: Arc<AtomicBool>,
}

impl PartialEq for WFInvAlecaframeSource {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

fn default_stop_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

const COMPONENT: &str = "WFInventory";

impl WFInvAlecaframeSource {
    pub fn get_default_path() -> PathBuf {
        crate::helper::get_local_data_path()
            .join("AlecaFrame")
            .join("lastData.dat")
    }
}

impl InventorySource for WFInvAlecaframeSource {
    fn update(&self, root: &Arc<Mutex<WarframeRootObject>>) -> Result<(), Error> {
        let path = if self.path.is_empty() {
            WFInvAlecaframeSource::get_default_path()
        } else {
            PathBuf::from(&self.path)
        };
        let parsed = block_on_async(load_inventory(&path))?;

        let mut root = root.lock().map_err(|_| {
            Error::new(
                "WFInvAlecaframeSource:Lock",
                "Root mutex poisoned",
                get_location!(),
            )
        })?;
        info(
            format!("{}:Update:Complete", COMPONENT),
            "Data file modified - root updated",
            &LoggerOptions::default(),
        );
        *root = parsed;
        Ok(())
    }

    fn start(&self, root: &Arc<Mutex<WarframeRootObject>>) {
        let source = self.clone();
        let root = root.clone();
        thread::spawn(move || {
            let path = if source.path.is_empty() {
                WFInvAlecaframeSource::get_default_path()
            } else {
                PathBuf::from(&source.path)
            };

            // Check if file exists before starting
            if !path.exists() {
                warning(
                    format!("{}:Watcher", COMPONENT),
                    format!("Inventory data file not found at: {}", path.display()),
                    &LoggerOptions::default(),
                );
            }

            let mut last_modified = fs::metadata(&path).and_then(|m| m.modified()).ok();

            // Initial load if file exists
            if path.exists() {
                if let Err(e) = source.update(&root) {
                    e.log("WFInventoryState.log").with_location(get_location!());
                }
            }

            loop {
                thread::sleep(Duration::from_millis(500));

                if source.stop_flag.load(Ordering::Relaxed) {
                    info(
                        format!("{}:Watcher", COMPONENT),
                        "Watcher stopped",
                        &LoggerOptions::default(),
                    );
                    break;
                }

                match fs::metadata(&path).and_then(|m| m.modified()) {
                    Ok(modified) => {
                        if last_modified.is_none_or(|last| modified > last) {
                            last_modified = Some(modified);
                            if let Err(e) = source.update(&root) {
                                e.log("WFInventoryState.log").with_location(get_location!());
                            }
                        }
                    }
                    Err(_) => {
                        // File doesn't exist or can't be accessed - silently skip
                        // Reset last_modified so we catch it when it appears
                        if last_modified.is_some() {
                            last_modified = None;
                            warning(
                                format!("{}:Watcher", COMPONENT),
                                format!(
                                    "Inventory data file no longer accessible at: {}",
                                    path.display()
                                ),
                                &LoggerOptions::default(),
                            );
                        }
                    }
                }
            }
        });
    }

    fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    fn validate(&self) -> Result<(), Error> {
        let path = if self.path.is_empty() {
            WFInvAlecaframeSource::get_default_path()
        } else {
            PathBuf::from(&self.path)
        };

        if !path.exists() {
            return Err(Error::new(
                "WFInvAlecaframeSource:Validate",
                format!("Inventory data file not found at: {}", path.display()),
                get_location!(),
            ));
        }

        Ok(())
    }
}

/* ========================== */
/*        HELPERS             */
/* ========================== */

async fn load_inventory(path: &Path) -> Result<WarframeRootObject, Error> {
    let bytes = read_file(path)?;
    let data = decrypt_lastdata(&bytes).await?;
    let parsed = parse_lastdata(&data)?;
    Ok(parsed)
}

fn read_file(path: &Path) -> Result<Vec<u8>, Error> {
    let mut file = File::open(path).map_err(|e| {
        Error::from_io(
            &format!("{COMPONENT}:Open"),
            &PathBuf::from(path),
            "Failed to open file",
            e,
            get_location!(),
        )
    })?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| {
        Error::from_io(
            &format!("{COMPONENT}:Read"),
            &PathBuf::from(path),
            "Failed to read file",
            e,
            get_location!(),
        )
    })?;

    Ok(buf)
}

/// Environment overrides for the AlecaFrame decryption key and IV, each 32 hex
/// characters. These outrank the persisted settings so a build can be pointed at
/// different keys without editing anyone's settings file.
pub const DECRYPT_KEY_ENV: &str = "QF_WF_DECRYPT_KEY";
pub const DECRYPT_IV_ENV: &str = "QF_WF_DECRYPT_IV";

fn parse_hex16(value: &str) -> Option<[u8; 16]> {
    let value = value.trim();
    if value.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(value.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

fn env_or_setting(env: &str, setting: &str) -> Option<[u8; 16]> {
    std::env::var(env)
        .ok()
        .as_deref()
        .and_then(parse_hex16)
        .or_else(|| parse_hex16(setting))
}

/// Key and IV from local configuration, or None to fall back to the API.
///
/// Reads the project-root `config.json` (see `crate::config`), with the
/// environment taking precedence. Deliberately not the app's `settings.json`:
/// that lives in the OS app-data directory and is seeded from upstream
/// Quantframe's own settings, so a re-seed would wipe these.
///
/// Returns None unless BOTH are configured and valid - a half-configured pair is
/// almost certainly a typo, and silently falling back would make that look like
/// the 403 it was meant to avoid.
fn local_decrypt_keys() -> Option<([u8; 16], [u8; 16])> {
    let config = crate::config::get();
    let key = env_or_setting(
        DECRYPT_KEY_ENV,
        config.wf_decrypt_key.as_deref().unwrap_or_default(),
    );
    let iv = env_or_setting(
        DECRYPT_IV_ENV,
        config.wf_decrypt_iv.as_deref().unwrap_or_default(),
    );
    match (key, iv) {
        (Some(key), Some(iv)) => Some((key, iv)),
        (None, None) => None,
        _ => {
            warning(
                "DecryptLastData:LocalKeys",
                "Only one of the decryption key and IV is set (or one is not 32 hex \
                 characters); ignoring both and falling back to the API",
                &LoggerOptions::default(),
            );
            None
        }
    }
}

async fn decrypt_lastdata(data: &[u8]) -> Result<String, Error> {
    let af_api = states::app_state()?;

    // Prefer locally configured keys. The API endpoint is entitlement-gated and
    // answers 403 for accounts without it, so for those this is the only route.
    let (key, iv) = match local_decrypt_keys() {
        Some(pair) => {
            info(
                "DecryptLastData:LocalKeys",
                "Using locally configured decryption key",
                &LoggerOptions::default(),
            );
            pair
        }
        None => {
            let keys = match af_api.qf_client.alecaframe().get_decrypt_keys().await {
                Ok(keys) => keys,
                Err(err) => {
                    return Err(Error::new(
                        "DecryptLastData:GetKeys",
                        format!(
                            "Failed to get decrypt keys: {err:?}. A 403 here means the \
                             account lacks the entitlement - set wf_decrypt_key and \
                             wf_decrypt_iv in the project-root config.json to decrypt \
                             locally instead (see docs/FORK.md)."
                        ),
                        get_location!(),
                    ))
                }
            };
            let key: [u8; 16] = keys.key.as_slice().try_into().map_err(|_| {
                Error::new(
                    "DecryptLastData:KeySize",
                    "Key must be 16 bytes",
                    get_location!(),
                )
            })?;
            let iv: [u8; 16] = keys.iv.as_slice().try_into().map_err(|_| {
                Error::new(
                    "DecryptLastData:IvSize",
                    "IV must be 16 bytes",
                    get_location!(),
                )
            })?;
            (key, iv)
        }
    };

    let decrypted = DecryptThingy::new(&key.into(), &iv.into())
        .decrypt_padded_vec_mut::<NoPadding>(data)
        .map_err(|e| {
            Error::new(
                "DecryptLastData:Decrypt",
                format!("Decrypt failed: {e:?}"),
                get_location!(),
            )
        })?;

    String::from_utf8(decrypted).map_err(|e| {
        Error::new(
            "DecryptLastData:Utf8",
            format!("UTF-8 parse failed: {e:?}"),
            get_location!(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::parse_hex16;

    #[test]
    fn parses_32_hex_characters() {
        let parsed = parse_hex16("000102030405060708090a0b0c0d0e0f").expect("should parse");
        assert_eq!(
            parsed,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    #[test]
    fn accepts_surrounding_whitespace_and_uppercase() {
        let lower = parse_hex16("0a1b2c3d4e5f60718293a4b5c6d7e8f9");
        let upper = parse_hex16("  0A1B2C3D4E5F60718293A4B5C6D7E8F9\n");
        assert!(lower.is_some());
        assert_eq!(lower, upper);
    }

    /// A wrong-length or non-hex value must be rejected rather than silently
    /// padded or truncated: a bad key would decrypt to garbage, which the parser
    /// downstream would report as malformed inventory rather than a bad key.
    #[test]
    fn rejects_anything_that_is_not_exactly_16_bytes_of_hex() {
        for bad in [
            "",                                   // unset
            "0a1b2c3d",                           // too short
            "000102030405060708090a0b0c0d0e0f00", // too long
            "0a1b2c3d4e5f60718293a4b5c6d7e8fg",   // 'g' is not hex
            "0a1b2c3d4e5f60718293a4b5c6d7e8f ",   // 31 hex + space
        ] {
            assert!(
                parse_hex16(bad).is_none(),
                "{bad:?} should have been rejected"
            );
        }
    }
}
