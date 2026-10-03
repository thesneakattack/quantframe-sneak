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
//! 2. `config.json` in the project root
//! 3. the compiled default
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

    /// AES-128-CBC key and IV, 32 hex characters each, used to decrypt
    /// AlecaFrame's `lastData.dat` locally. Absent means "ask the API", which
    /// is the upstream behaviour and returns 403 without the entitlement.
    pub wf_decrypt_key: Option<String>,
    pub wf_decrypt_iv: Option<String>,
}

/// Where the config file is, if there is one.
///
/// `explicit` short-circuits the search; otherwise walk up from `start`
/// looking for `config.json`.
fn resolve_config_path(start: &Path, explicit: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return if path.is_file() { Some(path) } else { None };
    }
    let mut dir = Some(start);
    for _ in 0..=MAX_ASCENT {
        let candidate = dir?.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = dir?.parent();
    }
    None
}

fn load() -> FileConfig {
    let explicit = std::env::var(CONFIG_PATH_ENV).ok().map(PathBuf::from);
    let explicit_was_set = explicit.is_some();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let Some(path) = resolve_config_path(&cwd, explicit) else {
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

    #[test]
    fn parses_the_three_fork_settings() {
        let parsed: FileConfig = serde_json::from_str(
            r#"{"qf_api_url":"http://localhost:6969","wf_decrypt_key":"aa","wf_decrypt_iv":"bb"}"#,
        )
        .expect("should parse");
        assert_eq!(parsed.qf_api_url.as_deref(), Some("http://localhost:6969"));
        assert_eq!(parsed.wf_decrypt_key.as_deref(), Some("aa"));
        assert_eq!(parsed.wf_decrypt_iv.as_deref(), Some("bb"));
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

        let found = resolve_config_path(&src_tauri, None).expect("should find the parent config");
        assert_eq!(found, config);
    }

    #[test]
    fn prefers_a_config_in_the_working_directory_itself() {
        let root = tempfile::tempdir().expect("tempdir");
        let here = root.path().join("config.json");
        fs::write(&here, "{}").expect("write");

        let found = resolve_config_path(root.path(), None).expect("should find it");
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

        let found = resolve_config_path(root.path(), Some(explicit.clone())).expect("should find");
        assert_eq!(found, explicit);
    }

    /// No config at all is the normal case for a packaged build: it must be
    /// absent, not an error.
    #[test]
    fn returns_none_when_no_config_exists() {
        let root = tempfile::tempdir().expect("tempdir");
        let empty = root.path().join("nothing");
        fs::create_dir(&empty).expect("mkdir");
        assert!(resolve_config_path(&empty, None).is_none());
    }
}
