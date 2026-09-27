use serde::{Deserialize, Serialize};

/// A named page that can be displayed on the screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub duration: i64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Payload used when creating a page.
#[derive(Debug, Clone, Deserialize)]
pub struct NewPage {
    pub name: String,
    pub url: String,
    #[serde(default = "default_duration")]
    pub duration: i64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Payload used when updating a page (all fields optional).
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePage {
    pub name: Option<String>,
    pub url: Option<String>,
    pub duration: Option<i64>,
    pub enabled: Option<bool>,
}

fn default_duration() -> i64 {
    60
}

fn default_true() -> bool {
    true
}

/// Schedule types supported by the player.
pub const SCHEDULE_TIME: &str = "TIME";
pub const SCHEDULE_ROTATION: &str = "ROTATION";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: i64,
    pub page_id: i64,
    pub schedule_type: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    /// Comma separated days, e.g. "MON,TUE,WED".
    pub days: Option<String>,
    pub priority: i64,
    pub enabled: bool,
    /// Ordering within a rotation schedule.
    pub sequence: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewSchedule {
    pub page_id: i64,
    #[serde(default = "default_schedule_type")]
    pub schedule_type: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    #[serde(default)]
    pub days: Option<Vec<String>>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub sequence: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSchedule {
    pub page_id: Option<i64>,
    pub schedule_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub days: Option<Vec<String>>,
    pub priority: Option<i64>,
    pub enabled: Option<bool>,
    pub sequence: Option<i64>,
}

fn default_schedule_type() -> String {
    SCHEDULE_TIME.to_string()
}

/// Persisted display state — the single source of truth for what is showing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayState {
    pub id: i64,
    pub current_page_id: Option<i64>,
    pub current_url: Option<String>,
    pub mode: String,
    pub started_at: Option<String>,
    pub expires_at: Option<String>,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            id: 1,
            current_page_id: None,
            current_url: None,
            mode: "manual".to_string(),
            started_at: None,
            expires_at: None,
        }
    }
}

/// Display modes.
pub const MODE_MANUAL: &str = "manual";
pub const MODE_SCHEDULER: &str = "scheduler";
pub const MODE_OVERRIDE: &str = "override";
/// Display cleared by the operator: show the default standby screen until
/// content becomes available again (manual URL, override, or a schedule).
pub const MODE_IDLE: &str = "idle";

/// Device identity record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub hostname: Option<String>,
    pub platform: Option<String>,
    pub version: Option<String>,
    pub location: Option<String>,
    pub group_name: Option<String>,
    pub last_seen: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateDevice {
    pub name: Option<String>,
    pub location: Option<String>,
    pub group_name: Option<String>,
}

/// Generic command record for auditing remote commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
    pub id: i64,
    pub command: String,
    pub payload: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub executed_at: Option<String>,
}