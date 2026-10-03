use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AdvancedSettings {
    pub http_server: HttpServerSettings,

    /// Overrides the Quantframe API base URL. Empty means "use the compiled
    /// default", which is the production endpoint for release builds and
    /// localhost:6969 for dev builds. The QF_API_URL environment variable wins
    /// over this. See qf_api::client::API_URL_ENV.
    ///
    /// Defaulted so settings files written before this field existed still load.
    #[serde(default)]
    pub qf_api_url: String,

    /// AES-128-CBC key and IV, as 32 hex characters each, used to decrypt
    /// AlecaFrame's `lastData.dat` locally.
    ///
    /// Empty means "ask the API", which is the upstream behaviour. That endpoint
    /// (`/alecaframe/decrypt-keys`) returns 403 on accounts without the
    /// entitlement, which is the only reason these exist. The values are static
    /// and are deliberately not shipped in this repository; see docs/FORK.md.
    ///
    /// `QF_WF_DECRYPT_KEY` / `QF_WF_DECRYPT_IV` take precedence over these.
    #[serde(default)]
    pub wf_decrypt_key: String,

    #[serde(default)]
    pub wf_decrypt_iv: String,
}
