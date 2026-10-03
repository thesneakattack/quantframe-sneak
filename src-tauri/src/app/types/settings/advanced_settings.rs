use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AdvancedSettings {
    pub http_server: HttpServerSettings,
}

// The fork's own settings - the Quantframe API base URL and the AlecaFrame
// decryption key and IV - deliberately do NOT live here. This struct is
// serialised into the app-data `settings.json`, which `seed-from-upstream.sh`
// copies from upstream Quantframe's settings, so anything stored here can be
// clobbered by a re-seed and is easy to confuse with the upstream app's state.
// They live in a gitignored `config.json` in the project root instead; see
// `crate::config`.
