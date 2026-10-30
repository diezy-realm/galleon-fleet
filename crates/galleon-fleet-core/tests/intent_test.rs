//! Integration tests for Quartermaster intent classification and deterministic responses (TASK-079).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use galleon_fleet_core::{
    DiagnosticItem, FleetMetrics, FleetStore, Intent, IntentClassification,
    QuartermasterChatResponse, SuggestedAction, answer_engine_room, answer_report, classify,
    classify_intent,
};
use serde_json::json;

/// Test parity with Go table test cases from `engine/src/fleet/quartermaster_router_test.go`.
#[test]
fn test_classify_intent_go_parity_table() {
    let cases = vec![
        ("apa itu Galleon?", Intent::Chat),
        ("berapa RAM engine sekarang?", Intent::EngineRoom),
        ("ringkas status semua ship", Intent::Report),
        ("siapkan rilis v1.4", Intent::Objective),
        ("kerjakan quest baru untuk dev ship", Intent::Objective),
        ("halo quartermaster", Intent::Chat),
    ];

    for (prompt, want) in cases {
        let got_intent = classify(prompt);
        assert_eq!(
            got_intent, want,
            "classify({prompt:?}) = {got_intent:?}, want {want:?}"
        );

        let got_classification = classify_intent(prompt);
        assert_eq!(
            got_classification.intent, want,
            "classify_intent({prompt:?}).intent = {:?}, want {want:?}",
            got_classification.intent
        );
        assert!(
            got_classification.confidence > 0.0,
            "classify_intent({prompt:?}) confidence must be > 0, got {}",
            got_classification.confidence
        );
        assert!(
            !got_classification.reason.is_empty(),
            "classify_intent({prompt:?}) reason must not be empty"
        );
    }
}

/// Verify exact confidence scores and reasons matching Go implementation.
#[test]
fn test_classify_intent_confidence_and_reasons() {
    let objective = classify_intent("siapkan quest baru");
    assert_eq!(objective.intent, Intent::Objective);
    assert!((objective.confidence - 0.8).abs() < 1e-6);
    assert_eq!(objective.reason, "matched objective/quest verb");

    let engine_room = classify_intent("cek memory dan cpu");
    assert_eq!(engine_room.intent, Intent::EngineRoom);
    assert!((engine_room.confidence - 0.8).abs() < 1e-6);
    assert_eq!(engine_room.reason, "matched engine-room/resource term");

    let report = classify_intent("laporan progress armada");
    assert_eq!(report.intent, Intent::Report);
    assert!((report.confidence - 0.75).abs() < 1e-6);
    assert_eq!(report.reason, "matched report/status term");

    let chat = classify_intent("siapakah kamu?");
    assert_eq!(chat.intent, Intent::Chat);
    assert!((chat.confidence - 0.5).abs() < 1e-6);
    assert_eq!(chat.reason, "no specific signal; default chat");
}

/// Verify priority order: Objective -> EngineRoom -> Report -> Chat.
#[test]
fn test_classify_priority_order() {
    // Both objective ("deploy") and engine room ("cpu", "ram") present -> Objective wins
    assert_eq!(
        classify("deploy service baru dengan 4GB ram dan 2 cpu"),
        Intent::Objective
    );

    // Both engine room ("health", "uptime") and report ("status", "report") present -> EngineRoom wins
    assert_eq!(
        classify("laporkan status health dan uptime engine"),
        Intent::EngineRoom
    );

    // Both report ("ringkas") and chat elements present -> Report wins
    assert_eq!(
        classify("tolong ringkas situasi fleet hari ini"),
        Intent::Report
    );
}

/// Verify case-insensitivity, trim handling, and edge cases.
#[test]
fn test_classify_edge_cases() {
    assert_eq!(classify(""), Intent::Chat);
    assert_eq!(classify("   \t\n  "), Intent::Chat);
    assert_eq!(classify("  SIAPKAN RILIS V1.4  "), Intent::Objective);
    assert_eq!(classify("BERAPA RAM SEKARANG?"), Intent::EngineRoom);
    assert_eq!(classify("  ringkas status SEMUA SHIP  "), Intent::Report);

    // Objective keywords
    for kw in &[
        "quest",
        "misi",
        "mission",
        "siapkan",
        "prepare",
        "build",
        "implement",
        "deploy",
        "rilis",
        "release",
        "kerjakan",
        "execute",
        "buatkan",
        "rencana",
        "rencanakan",
        "plan",
    ] {
        assert_eq!(
            classify(kw),
            Intent::Objective,
            "keyword '{kw}' should match Objective"
        );
    }

    // EngineRoom keywords
    for kw in &[
        "ram",
        "cpu",
        "memory",
        "memori",
        "engine room",
        "engine-room",
        "health",
        "uptime",
        "budget",
        "biaya",
        "cost",
        "resource",
        "diagnos",
        "provider health",
    ] {
        assert_eq!(
            classify(kw),
            Intent::EngineRoom,
            "keyword '{kw}' should match EngineRoom"
        );
    }

    // Report keywords
    for kw in &[
        "status",
        "laporan",
        "report",
        "ringkas",
        "summary",
        "summarize",
        "briefing",
        "progress",
        "semua ship",
        "all ships",
        "fleet report",
    ] {
        assert_eq!(
            classify(kw),
            Intent::Report,
            "keyword '{kw}' should match Report"
        );
    }
}

/// Verify intent serialization / display.
#[test]
fn test_intent_serialization_and_display() {
    assert_eq!(Intent::Chat.as_str(), "chat");
    assert_eq!(Intent::EngineRoom.as_str(), "engine_room");
    assert_eq!(Intent::Report.as_str(), "report");
    assert_eq!(Intent::Objective.as_str(), "objective");

    assert_eq!(format!("{}", Intent::Chat), "chat");
    assert_eq!(format!("{}", Intent::EngineRoom), "engine_room");
    assert_eq!(format!("{}", Intent::Report), "report");
    assert_eq!(format!("{}", Intent::Objective), "objective");

    let json_val = serde_json::to_string(&Intent::EngineRoom).unwrap();
    assert_eq!(json_val, "\"engine_room\"");

    let deserialized: Intent = serde_json::from_str("\"engine_room\"").unwrap();
    assert_eq!(deserialized, Intent::EngineRoom);
}

/// Verify deterministic answer for `Intent::Report` with exact match to Go fixture.
#[test]
fn test_answer_report_parity_with_go_fixture() {
    let metrics = FleetMetrics {
        active_vessels: 3,
        underway_quests: 1,
        pending_approvals: 2,
        total_treasury_ledgers: 0,
        total_artifacts: 2,
        total_specialists: 0,
        total_squads: 0,
        system_uptime: Some("99.98%".to_string()),
        gateway_latency_ms: None,
        active_workers: None,
        memory_used_mb: None,
    };

    let resp = answer_report(&metrics, None);
    assert_eq!(
        resp.reply,
        "Fleet Report: 3 active vessels, 1 underway quests, 2 pending approvals, 2 artifacts in Treasury. Uptime 99.98%."
    );

    let actions = resp.suggested_actions.as_ref().unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].label, "Open Mission Board");
    assert_eq!(actions[0].action_type, "navigate");
    assert_eq!(actions[0].payload, Some(json!({ "tab": "mission-board" })));

    // Load and compare against fixture file directly
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/go-fleet/POST_api_chat_quartermaster.json");

    if fixture_path.exists() {
        let fixture_bytes = std::fs::read(&fixture_path).unwrap();
        let fixture_json: serde_json::Value = serde_json::from_slice(&fixture_bytes).unwrap();

        let resp_json = serde_json::to_value(&resp).unwrap();
        assert_eq!(resp_json["reply"], fixture_json["reply"]);
        assert_eq!(
            resp_json["suggestedActions"],
            fixture_json["suggestedActions"]
        );
    }
}

/// Verify deterministic answer for `Intent::Report` with honest fallback uptime.
#[test]
fn test_answer_report_unmeasured_uptime() {
    let metrics = FleetMetrics {
        active_vessels: 5,
        underway_quests: 2,
        pending_approvals: 0,
        total_treasury_ledgers: 10,
        total_artifacts: 7,
        total_specialists: 3,
        total_squads: 1,
        system_uptime: None,
        gateway_latency_ms: None,
        active_workers: None,
        memory_used_mb: None,
    };

    let resp = answer_report(&metrics, None);
    assert_eq!(
        resp.reply,
        "Fleet Report: 5 active vessels, 2 underway quests, 0 pending approvals, 7 artifacts in Treasury. Uptime —."
    );

    // With explicit uptime override
    let resp_override = answer_report(&metrics, Some("3h 45m"));
    assert_eq!(
        resp_override.reply,
        "Fleet Report: 5 active vessels, 2 underway quests, 0 pending approvals, 7 artifacts in Treasury. Uptime 3h 45m."
    );
}

/// Verify deterministic answer for `Intent::EngineRoom`.
#[test]
fn test_answer_engine_room() {
    let metrics = FleetMetrics {
        active_vessels: 3,
        underway_quests: 1,
        pending_approvals: 2,
        total_treasury_ledgers: 0,
        total_artifacts: 2,
        total_specialists: 0,
        total_squads: 0,
        system_uptime: None,
        gateway_latency_ms: Some(14.0),
        active_workers: Some(4),
        memory_used_mb: Some(128.5),
    };

    let diags = vec![
        DiagnosticItem::new("d-1", "workspace", "healthy", "—", "ok"),
        DiagnosticItem::new("d-2", "daemon", "healthy", "—", "ok"),
        DiagnosticItem::new("d-3", "database", "warning", "—", "high latency"),
    ];

    let resp = answer_engine_room(&metrics, &diags);
    assert_eq!(
        resp.reply,
        "Engine Room: 2/3 subsystems healthy, gateway latency 14ms, memory 128.5 MB, 4 active workers."
    );

    let actions = resp.suggested_actions.as_ref().unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].label, "View Crow's Nest");
    assert_eq!(actions[0].action_type, "navigate");
    assert_eq!(actions[0].payload, Some(json!({ "tab": "crows-nest" })));

    // Test honest unmeasured signals
    let honest_metrics = FleetMetrics {
        active_vessels: 3,
        underway_quests: 1,
        pending_approvals: 2,
        total_treasury_ledgers: 0,
        total_artifacts: 2,
        total_specialists: 0,
        total_squads: 0,
        system_uptime: None,
        gateway_latency_ms: None,
        active_workers: None,
        memory_used_mb: Some(64.0),
    };

    let honest_resp = answer_engine_room(&honest_metrics, &diags);
    assert_eq!(
        honest_resp.reply,
        "Engine Room: 2/3 subsystems healthy, gateway latency —, memory 64.0 MB, — active workers."
    );
}

/// Verify constructors and serialization of SuggestedAction and QuartermasterChatResponse.
#[test]
fn test_models_constructors_and_serialization() {
    let action_no_payload = SuggestedAction::new("Open Settings", "open_tab");
    assert_eq!(action_no_payload.label, "Open Settings");
    assert_eq!(action_no_payload.action_type, "open_tab");
    assert!(action_no_payload.payload.is_none());

    let val_action = serde_json::to_value(&action_no_payload).unwrap();
    assert_eq!(val_action["label"], "Open Settings");
    assert_eq!(val_action["actionType"], "open_tab");
    assert!(val_action.get("payload").is_none());

    let action_with_payload =
        SuggestedAction::with_payload("Open Ship", "navigate", json!({ "shipId": "ship-1" }));
    assert_eq!(
        action_with_payload.payload,
        Some(json!({ "shipId": "ship-1" }))
    );

    let resp_basic = QuartermasterChatResponse::new("Hello Pirate King!");
    assert_eq!(resp_basic.reply, "Hello Pirate King!");
    assert!(resp_basic.suggested_actions.is_none());
    assert!(resp_basic.generated_artifact_preview.is_none());

    let resp_with_actions = QuartermasterChatResponse::with_suggested_actions(
        "Ready to serve.",
        vec![action_with_payload],
    );
    let val_resp = serde_json::to_value(&resp_with_actions).unwrap();
    assert_eq!(val_resp["reply"], "Ready to serve.");
    assert_eq!(val_resp["suggestedActions"].as_array().unwrap().len(), 1);

    let classification = IntentClassification {
        intent: Intent::Chat,
        confidence: 0.95,
        reason: "explicit test reason".to_string(),
    };
    let val_cls = serde_json::to_value(&classification).unwrap();
    assert_eq!(val_cls["intent"], "chat");
    assert_eq!(val_cls["reason"], "explicit test reason");
}

/// Verify FleetStore integration for answering report and engine room.
#[test]
fn test_fleet_store_answer_helpers() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = FleetStore::new(temp_dir.path());

    // Answer report on empty store
    let report_resp = store.answer_report(Some(42.0), Some("1h 20m"));
    assert_eq!(
        report_resp.reply,
        "Fleet Report: 0 active vessels, 0 underway quests, 0 pending approvals, 0 artifacts in Treasury. Uptime 1h 20m."
    );

    // Answer engine room on empty store
    let diags = vec![DiagnosticItem::new("d-1", "core", "healthy", "—", "ok")];
    let engine_resp = store.answer_engine_room(Some(42.0), &diags);
    assert_eq!(
        engine_resp.reply,
        "Engine Room: 1/1 subsystems healthy, gateway latency —, memory 42.0 MB, — active workers."
    );
}
