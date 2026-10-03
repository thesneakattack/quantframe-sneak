use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LogSettings {
    pub ee_log_path: String,
}
