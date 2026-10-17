use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum CronShellOutputFormat {
    #[default]
    Wrapped,
    Raw,
}

pub fn deserialize_maybe_stringified<T: serde::de::DeserializeOwned>(
    v: &serde_json::Value,
) -> Result<T, serde_json::Error> {
    // Fast path: value is already the right shape (object, array, etc.)
    match serde_json::from_value::<T>(v.clone()) {
        Ok(parsed) => Ok(parsed),
        Err(first_err) => {
            // If it's a string, try parsing the string as JSON first.
            if let Some(s) = v.as_str() {
                let s = s.trim();
                if (s.starts_with('{') || s.starts_with('['))
                    && let Ok(inner) = serde_json::from_str::<serde_json::Value>(s)
                {
                    return serde_json::from_value::<T>(inner);
                }
            }
            Err(first_err)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    #[default]
    Shell,
    Agent,
}

impl From<JobType> for &'static str {
    fn from(value: JobType) -> Self {
        match value {
            JobType::Shell => "shell",
            JobType::Agent => "agent",
        }
    }
}

impl TryFrom<&str> for JobType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "shell" => Ok(JobType::Shell),
            "agent" => Ok(JobType::Agent),
            _ => Err(format!(
                "Invalid job type '{}'. Expected one of: 'shell', 'agent'",
                value
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SessionTarget {
    #[default]
    Isolated,
    Main,
}

impl SessionTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
            Self::Main => "main",
        }
    }

    /// Parse a session target, treating any non-`main` value (including typos)
    /// as isolated. Prefer [`Self::try_parse`] at user-input boundaries so an
    /// invalid value is rejected instead of silently becoming isolated.
    pub fn parse(raw: &str) -> Self {
        if raw.eq_ignore_ascii_case("main") {
            Self::Main
        } else {
            Self::Isolated
        }
    }

    /// Parse a session target from user input. `isolated` and `main` are
    /// accepted case-insensitively after trim; anything else is an error.
    pub fn try_parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "isolated" => Ok(Self::Isolated),
            "main" => Ok(Self::Main),
            _ => Err(format!(
                "Invalid session_target '{trimmed}'. Expected one of: 'isolated', 'main'"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Schedule {
    Cron {
        expr: String,
        #[serde(default)]
        tz: Option<String>,
    },
    At {
        at: DateTime<Utc>,
    },
    Every {
        every_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryConfig {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default = "default_true")]
    pub best_effort: bool,
}

impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            mode: "none".to_string(),
            channel: None,
            to: None,
            thread_id: None,
            best_effort: true,
        }
    }
}

pub fn default_true() -> bool {
    true
}

fn default_source() -> String {
    "imperative".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub expression: String,
    pub schedule: Schedule,
    pub command: String,
    pub prompt: Option<String>,
    pub name: Option<String>,
    pub job_type: JobType,
    pub session_target: SessionTarget,
    pub model: Option<String>,
    #[serde(default)]
    pub agent_alias: String,
    pub enabled: bool,
    pub delivery: DeliveryConfig,
    pub delete_after_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default = "default_true")]
    pub uses_memory: bool,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub shell_output_format: CronShellOutputFormat,
    pub created_at: DateTime<Utc>,
    pub next_run: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub last_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronRun {
    pub id: i64,
    pub job_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub status: String,
    pub output: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CronJobPatch {
    pub schedule: Option<Schedule>,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub delivery: Option<DeliveryConfig>,
    pub model: Option<String>,
    pub session_target: Option<SessionTarget>,
    pub delete_after_run: Option<bool>,
    pub allowed_tools: Option<Vec<String>>,
    pub uses_memory: Option<bool>,
    pub shell_output_format: Option<CronShellOutputFormat>,
}

impl ::clawcrew_api::attribution::Attributable for CronJob {
    fn role(&self) -> ::clawcrew_api::attribution::Role {
        let kind = match self.schedule {
            Schedule::Cron { .. } => ::clawcrew_api::attribution::CronKind::Cron,
            Schedule::At { .. } => ::clawcrew_api::attribution::CronKind::At,
            Schedule::Every { .. } => ::clawcrew_api::attribution::CronKind::Interval,
        };
        ::clawcrew_api::attribution::Role::Cron(kind)
    }
    fn alias(&self) -> &str {
        &self.id
    }
}

pub fn validate_delivery_config(delivery: Option<&DeliveryConfig>) -> Result<()> {
    let Some(delivery) = delivery else {
        return Ok(());
    };

    if delivery.mode == "none" {
        return Ok(());
    }

    if delivery.mode != "announce" {
        bail!("unsupported delivery mode: {}", delivery.mode);
    }

    let channel = delivery.channel.as_deref().map(str::trim);
    if channel.filter(|value| !value.is_empty()).is_none() {
        bail!("delivery.channel is required for announce mode");
    }

    let has_target = delivery
        .to
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    if !has_target {
        bail!("delivery.to is required for announce mode");
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualCronRunResult {
    pub job_id: String,
    pub success: bool,
    pub status: String,
    pub output: String,
    pub duration_ms: i64,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}
