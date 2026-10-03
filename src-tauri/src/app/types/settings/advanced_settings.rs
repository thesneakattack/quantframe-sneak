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
}
