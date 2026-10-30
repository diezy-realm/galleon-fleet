//! Executive briefing domain types and calculations.
//!
//! Under SPEC-013 §Model Data (TASK-074):
//! - `GET /api/system/executive-briefing` provides sovereign fleet status report.
//! - `treasuryBurnRate` is honestly calculated from `treasuryLedger` or returns `None` (`null`).
//! - Unmeasured/honest metrics: no fabricated constants.

use serde::{Deserialize, Serialize};

use crate::store::FleetStore;

/// Default report title for executive briefing.
pub const DEFAULT_BRIEFING_TITLE: &str = "Sovereign Fleet Watch Report";

/// Executive briefing report snapshot for dashboard (`GET /api/system/executive-briefing`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutiveBriefing {
    pub active_alerts: Vec<String>,
    pub admiral_status: String,
    pub date: String,
    pub generated_at: String,
    pub key_achievements: Vec<String>,
    pub recommended_focus: String,
    pub title: String,
    pub treasury_burn_rate: Option<String>,
}

/// Calculate `treasuryBurnRate` from `treasuryLedger` collection.
///
/// Returns `None` (`null` in JSON) if the collection is absent, empty,
/// or contains no valid costs.
pub fn calculate_treasury_burn_rate(store: &FleetStore) -> Option<String> {
    match store.get_collection("treasuryLedger") {
        Ok(Some(bytes)) => {
            if let Ok(serde_json::Value::Array(arr)) = serde_json::from_slice(&bytes) {
                if arr.is_empty() {
                    return None;
                }
                let mut total_cost: f64 = 0.0;
                let mut found_cost = false;
                for item in &arr {
                    if let Some(cost) = item.get("costUSD").and_then(|v| v.as_f64()) {
                        total_cost += cost;
                        found_cost = true;
                    }
                }
                if found_cost {
                    Some(format!("${total_cost:.2}"))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Calculate `ExecutiveBriefing` from a `FleetStore` and given date and timestamp strings.
pub fn calculate_executive_briefing(
    store: &FleetStore,
    date: String,
    generated_at: String,
) -> ExecutiveBriefing {
    let artifacts = store.count_collection("artifacts");
    let underway = store.count_by_status("quests", &["underway", "in_progress", "active"]);
    let pending = store.count_by_status("approvals", &["pending"]);
    let specialists = store.count_collection("crew");

    let admiral_status = if pending > 0 {
        format!("Awaiting Captain — {pending} approval(s) pending")
    } else {
        "Operational Ready".to_string()
    };

    let key_achievements = vec![
        format!("{artifacts} code artifact(s) indexed into the Treasury Gallery"),
        format!("{specialists} specialist(s) commissioned across the fleet roster"),
        format!("{underway} voyage(s) currently underway"),
    ];

    let active_alerts = if pending > 0 {
        vec![format!(
            "{pending} pending Captain's Approval await signature"
        )]
    } else {
        vec![]
    };

    let recommended_focus = if pending > 0 {
        "Review and sign the pending approval(s), then reassign idle Ships to the quest backlog."
            .to_string()
    } else {
        "Patrol steady; no decisions pending.".to_string()
    };

    let treasury_burn_rate = calculate_treasury_burn_rate(store);

    ExecutiveBriefing {
        active_alerts,
        admiral_status,
        date,
        generated_at,
        key_achievements,
        recommended_focus,
        title: DEFAULT_BRIEFING_TITLE.to_string(),
        treasury_burn_rate,
    }
}
