//! Fork-specific configuration, read from a `config.json` in the project root.
//!
//! These settings are deployment configuration, not user preferences, so they
//! deliberately do **not** live in the app's `settings.json`. That file sits in
//! the OS app-data directory and is seeded from upstream Quantframe's own
//! settings by `scripts/seed-from-upstream.sh`, so anything written there is
//! liable to be overwritten by a re-seed and is easy to confuse with the
//! upstream app's state.
//!
//! Precedence for every value, highest first:
//!
//! 1. the environment variable (`QF_API_URL`, `WF_DECRYPT_KEY`, `WF_DECRYPT_IV`)
//! 2. `config.json` in the project root, for a checkout
//! 3. `config.json` in the app-data directory, for an installed build, which
//!    has no project root to walk up to
//! 4. the compiled default
//!
//! `config.json` is gitignored: it holds the AlecaFrame AES key and IV, which
//! are not shipped in this repository.

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use serde::Deserialize;
use utils::{info, warning, LoggerOptions};

static COMPONENT: &str = "Config";

/// Points at a config file explicitly, for packaged builds that have no
/// project root to walk up to.
pub const CONFIG_PATH_ENV: &str = "QF_CONFIG";

const CONFIG_FILE_NAME: &str = "config.json";

/// How far up from the working directory to look for the project root. `tauri
/// dev` runs from `src-tauri/`, so one level is enough in practice; a couple
/// more costs nothing and survives being run from a subdirectory.
const MAX_ASCENT: usize = 4;

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct FileConfig {
    /// Overrides the Quantframe API base URL. Absent means "use the compiled
    /// default": the production endpoint for release builds, localhost:6969
    /// for dev builds.
    pub qf_api_url: Option<String>,

    /// AES-128-CBC key and IV used to decrypt AlecaFrame's `lastData.dat`
    /// locally. Absent means "ask the API", which is the upstream behaviour
    /// and returns 403 without the entitlement.
    pub wf_decrypt_key: Option<KeyBytes>,
    pub wf_decrypt_iv: Option<KeyBytes>,
}

/// Sixteen key bytes, written either as the bytes themselves or as hex.
///
/// The byte list is the honest form: this is an encoding, not a hash, and
/// writing it as `[76, 69, ...]` makes that plain rather than implying the
/// value is protected. Hex is still accepted so an existing config keeps
/// working, and because it is what every published reference uses.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum KeyBytes {
    Bytes(Vec<u8>),
    Hex(String),
}

impl KeyBytes {
    /// The sixteen bytes, or None if this is not sixteen bytes' worth.
    ///
    /// A wrong-length or malformed key is rejected rather than padded: it
    /// would decrypt to rubbish, which the parser downstream reports as a
    /// broken inventory rather than a bad key.
    pub fn resolve(&self) -> Option<[u8; 16]> {
        match self {
            KeyBytes::Bytes(bytes) => bytes.as_slice().try_into().ok(),
            KeyBytes::Hex(text) => {
                let text = text.trim();
                if text.len() != 32 || !text.chars().all(|c| c.is_ascii_hexdigit()) {
                    return None;
                }
                let mut out = [0u8; 16];
                for (i, byte) in out.iter_mut().enumerate() {
                    *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
                }
                Some(out)
            }
        }
    }
}

/// Where the config file is, if there is one.
///
/// `explicit` short-circuits the search. Otherwise walk up from `start` for a
/// project root, then fall back to `app_data`.
///
/// The fallback is what makes an installed build work at all: it runs from
/// wherever its shortcut points, with no checkout above it, so the walk finds
/// nothing. Without somewhere else to look it would use no config, ask the API
/// for the AlecaFrame keys and get the 403 the local keys exist to avoid.
///
/// A checkout still wins, so running from source behaves as before. Putting
/// the installed copy's config in the app-data directory is safe even though
/// `settings.json` there is re-seeded from upstream Quantframe:
/// `scripts/seed-from-upstream.sh` copies a fixed list of files and
/// `config.json` is not one of them.
fn resolve_config_path(
    start: &Path,
    explicit: Option<PathBuf>,
    app_data: Option<&Path>,
) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return if path.is_file() { Some(path) } else { None };
    }
    // Walking off the top of the filesystem ends the walk, not the search:
    // the app-data fallback below is the whole point for an installed build,
    // which is exactly the case where the walk finds nothing.
    let mut dir = Some(start);
    for _ in 0..=MAX_ASCENT {
        let Some(current) = dir else {
            break;
        };
        let candidate = current.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = current.parent();
    }
    let candidate = app_data?.join(CONFIG_FILE_NAME);
    candidate.is_file().then_some(candidate)
}

fn load() -> FileConfig {
    let explicit = std::env::var(CONFIG_PATH_ENV).ok().map(PathBuf::from);
    let explicit_was_set = explicit.is_some();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let app_data = crate::helper::get_app_storage_path();
    let Some(path) = resolve_config_path(&cwd, explicit, Some(app_data.as_path())) else {
        if explicit_was_set {
            warning(
                format!("{}:Load", COMPONENT),
                format!("{} points at a file that does not exist", CONFIG_PATH_ENV),
                &LoggerOptions::default(),
            );
        }
        return FileConfig::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_json::from_str::<FileConfig>(&raw) {
            Ok(config) => {
                info(
                    format!("{}:Load", COMPONENT),
                    format!("Loaded {}", path.display()),
                    &LoggerOptions::default(),
                );
                config
            }
            Err(e) => {
                // A malformed config must not take the app down, but it must
                // be loud: silently falling back would look like the file was
                // being honoured.
                warning(
                    format!("{}:Load", COMPONENT),
                    format!("Ignoring {}: {}", path.display(), e),
                    &LoggerOptions::default(),
                );
                FileConfig::default()
            }
        },
        Err(e) => {
            warning(
                format!("{}:Load", COMPONENT),
                format!("Could not read {}: {}", path.display(), e),
                &LoggerOptions::default(),
            );
            FileConfig::default()
        }
    }
}

/// The project-root config, read once per process.
pub fn get() -> &'static FileConfig {
    static CONFIG: OnceLock<FileConfig> = OnceLock::new();
    CONFIG.get_or_init(load)
}

#[cfg(test)]
mod tests {
    use super::{resolve_config_path, FileConfig};
    use std::fs;

    /// The byte list is the form the config is meant to use.
    #[test]
    fn reads_a_key_written_as_bytes() {
        let parsed: FileConfig = serde_json::from_str(
            r#"{"wf_decrypt_key":[76,69,79,45,65,76,69,67,9,69,79,45,65,76,69,67]}"#,
        )
        .expect("should parse");
        assert_eq!(
            parsed.wf_decrypt_key.unwrap().resolve(),
            Some([76, 69, 79, 45, 65, 76, 69, 67, 9, 69, 79, 45, 65, 76, 69, 67])
        );
    }

    /// Hex still works, so a config written before this keeps running.
    #[test]
    fn still_reads_a_key_written_as_hex() {
        let parsed: FileConfig =
            serde_json::from_str(r#"{"wf_decrypt_key":"000102030405060708090a0b0c0d0e0f"}"#)
                .expect("should parse");
        assert_eq!(
            parsed.wf_decrypt_key.unwrap().resolve(),
            Some([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15])
        );
    }

    /// Anything that is not sixteen bytes is refused rather than padded; a
    /// wrong key decrypts to rubbish, which surfaces as a broken inventory.
    #[test]
    fn refuses_anything_that_is_not_sixteen_bytes() {
        for bad in [
            r#"{"wf_decrypt_key":[1,2,3]}"#,
            r#"{"wf_decrypt_key":"0a1b2c3d"}"#,
            r#"{"wf_decrypt_key":"0a1b2c3d4e5f60718293a4b5c6d7e8fg"}"#,
            r#"{"wf_decrypt_key":""}"#,
        ] {
            let parsed: FileConfig = serde_json::from_str(bad).expect("should parse");
            assert!(
                parsed.wf_decrypt_key.unwrap().resolve().is_none(),
                "{bad} should have been refused"
            );
        }
    }

    #[test]
    fn parses_the_three_fork_settings() {
        let parsed: FileConfig = serde_json::from_str(
            r#"{"qf_api_url":"http://localhost:6969","wf_decrypt_key":[1],"wf_decrypt_iv":[2]}"#,
        )
        .expect("should parse");
        assert_eq!(parsed.qf_api_url.as_deref(), Some("http://localhost:6969"));
        assert!(parsed.wf_decrypt_key.is_some());
        assert!(parsed.wf_decrypt_iv.is_some());
    }

    /// A config holding only one key must not fail the whole file; the others
    /// simply fall through to their own defaults.
    #[test]
    fn treats_absent_keys_as_unset_rather_than_failing() {
        let parsed: FileConfig =
            serde_json::from_str(r#"{"qf_api_url":"http://x"}"#).expect("should parse");
        assert_eq!(parsed.qf_api_url.as_deref(), Some("http://x"));
        assert!(parsed.wf_decrypt_key.is_none());
    }

    /// Unknown keys must be ignored, so a config written by a later version
    /// does not stop an older build from reading the keys it does know.
    #[test]
    fn ignores_unknown_keys() {
        let parsed: FileConfig =
            serde_json::from_str(r#"{"qf_api_url":"http://x","future_thing":1}"#)
                .expect("should parse");
        assert_eq!(parsed.qf_api_url.as_deref(), Some("http://x"));
    }

    /// `tauri dev` runs with the working directory at src-tauri/, so the
    /// project-root config is one level up. Both spellings must resolve.
    #[test]
    fn finds_config_in_the_directory_above_the_working_directory() {
        let root = tempfile::tempdir().expect("tempdir");
        let src_tauri = root.path().join("src-tauri");
        fs::create_dir(&src_tauri).expect("mkdir");
        let config = root.path().join("config.json");
        fs::write(&config, "{}").expect("write");

        let found =
            resolve_config_path(&src_tauri, None, None).expect("should find the parent config");
        assert_eq!(found, config);
    }

    #[test]
    fn prefers_a_config_in_the_working_directory_itself() {
        let root = tempfile::tempdir().expect("tempdir");
        let here = root.path().join("config.json");
        fs::write(&here, "{}").expect("write");

        let found = resolve_config_path(root.path(), None, None).expect("should find it");
        assert_eq!(found, here);
    }

    /// An explicit QF_CONFIG path wins over anything discovered by walking up,
    /// so a packaged build can be pointed at a config anywhere.
    #[test]
    fn an_explicit_override_wins_over_discovery() {
        let root = tempfile::tempdir().expect("tempdir");
        fs::write(root.path().join("config.json"), "{}").expect("write");
        let explicit = root.path().join("elsewhere.json");
        fs::write(&explicit, "{}").expect("write");

        let found =
            resolve_config_path(root.path(), Some(explicit.clone()), None).expect("should find");
        assert_eq!(found, explicit);
    }

    /// No config anywhere is not an error.
    #[test]
    fn returns_none_when_no_config_exists() {
        let root = tempfile::tempdir().expect("tempdir");
        let empty = root.path().join("nothing");
        fs::create_dir(&empty).expect("mkdir");
        assert!(resolve_config_path(&empty, None, None).is_none());
    }

    /// An installed build has no project root to walk up from - it runs from
    /// wherever the shortcut points. Without this it would find no config,
    /// fall back to the API for the AlecaFrame keys and get the 403 the local
    /// keys exist to avoid, so the inventory would never load.
    #[test]
    fn finds_the_config_an_installed_build_keeps_beside_its_data() {
        let root = tempfile::tempdir().expect("tempdir");
        let elsewhere = root.path().join("program files");
        let app_data = root.path().join("app data");
        fs::create_dir_all(&elsewhere).expect("mkdir");
        fs::create_dir_all(&app_data).expect("mkdir");
        fs::write(app_data.join("config.json"), "{}").expect("write");

        assert_eq!(
            resolve_config_path(&elsewhere, None, Some(app_data.as_path())),
            Some(app_data.join("config.json"))
        );
    }

    /// A checkout is what you are working on, so it wins over whatever the
    /// installed copy happens to have beside it.
    #[test]
    fn a_project_config_wins_over_the_installed_one() {
        let root = tempfile::tempdir().expect("tempdir");
        let project = root.path().join("project");
        let app_data = root.path().join("app data");
        fs::create_dir_all(&project).expect("mkdir");
        fs::create_dir_all(&app_data).expect("mkdir");
        fs::write(project.join("config.json"), "{}").expect("write");
        fs::write(app_data.join("config.json"), "{}").expect("write");

        assert_eq!(
            resolve_config_path(&project, None, Some(app_data.as_path())),
            Some(project.join("config.json"))
        );
    }
}
