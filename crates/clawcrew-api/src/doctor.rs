#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Ok,
    Warn,
    Error,
}

/// Structured diagnostic result for programmatic consumption (web dashboard, API).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiagResult {
    pub severity: Severity,
    pub category: String,
    pub message: String,
}

/// Runner trait decoupling callers (e.g. gateway API) from the concrete diagnostic engine.
pub trait DiagnosticsRunner: Send + Sync {
    fn run_diagnostics(&self) -> Vec<DiagResult>;
}
