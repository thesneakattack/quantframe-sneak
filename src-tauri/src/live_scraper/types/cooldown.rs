use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

fn default_duration() -> Duration {
    Duration::from_secs(0)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CooldownInfo {
    #[serde(default)]
    pub id: String,
    #[serde(default = "Utc::now")]
    pub start_time: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub end_time: DateTime<Utc>,
    #[serde(default)]
    pub cooldown_type: String,
    #[serde(default = "default_duration")]
    pub duration: Duration,
}

impl CooldownInfo {
    pub fn new(
        id: impl Into<String>,
        start_time: DateTime<Utc>,
        cooldown_type: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            id: id.into(),
            start_time,
            cooldown_type: cooldown_type.into(),
            duration,
            end_time: start_time
                + chrono::Duration::from_std(duration).expect("cooldown duration is valid"),
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.id, self.cooldown_type)
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.start_time
            + chrono::Duration::from_std(self.duration).expect("cooldown duration is valid")
    }

    pub fn remaining(&self) -> Option<Duration> {
        let remaining = self.expires_at() - Utc::now();

        if remaining.num_milliseconds() > 0 {
            remaining.to_std().ok()
        } else {
            None
        }
    }

    pub fn format_remaining(&self) -> String {
        if let Some(remaining) = self.remaining() {
            let seconds = remaining.as_secs();
            let minutes = seconds / 60;
            let hours = minutes / 60;

            if hours > 0 {
                format!("{}h {}m {}s", hours, minutes % 60, seconds % 60)
            } else if minutes > 0 {
                format!("{}m {}s", minutes, seconds % 60)
            } else {
                format!("{}s", seconds)
            }
        } else {
            "expired".to_string()
        }
    }
}

impl Default for CooldownInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            cooldown_type: String::new(),
            duration: Duration::from_secs(0),
        }
    }
}
