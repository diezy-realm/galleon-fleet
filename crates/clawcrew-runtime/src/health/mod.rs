pub use clawcrew_infra::health::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_component(prefix: &str) -> String {
        format!("{prefix}-{}", uuid::Uuid::new_v4())
    }

    #[test]
    fn mark_component_ok_initializes_component_state() {
        let component = unique_component("health-ok");

        mark_component_ok(&component);

        let snapshot = snapshot();
        let entry = snapshot
            .components
            .get(&component)
            .expect("component should be present after mark_component_ok");

        assert_eq!(entry.status, "ok");
        assert!(entry.last_ok.is_some());
        assert!(entry.last_error.is_none());
    }

    #[test]
    fn mark_component_error_then_ok_clears_last_error() {
        let component = unique_component("health-error");

        mark_component_error(&component, "first failure");
        let error_snapshot = snapshot();
        let errored = error_snapshot
            .components
            .get(&component)
            .expect("component should exist after mark_component_error");
        assert_eq!(errored.status, "error");
        assert_eq!(errored.last_error.as_deref(), Some("first failure"));

        mark_component_ok(&component);
        let recovered_snapshot = snapshot();
        let recovered = recovered_snapshot
            .components
            .get(&component)
            .expect("component should exist after recovery");
        assert_eq!(recovered.status, "ok");
        assert!(recovered.last_error.is_none());
        assert!(recovered.last_ok.is_some());
    }

    #[test]
    fn bump_component_restart_increments_counter() {
        let component = unique_component("health-restart");

        bump_component_restart(&component);
        bump_component_restart(&component);

        let snapshot = snapshot();
        let entry = snapshot
            .components
            .get(&component)
            .expect("component should exist after restart bump");

        assert_eq!(entry.restart_count, 2);
    }

    #[test]
    fn mark_component_starting_publishes_a_component_that_has_not_succeeded() {
        // The registry only creates an entry when a mutation reaches it, so a
        // component with nothing to report would otherwise be absent from
        // `/health` rather than visibly not-yet-healthy.
        let component = unique_component("health-starting");

        assert!(
            !snapshot().components.contains_key(&component),
            "precondition: the component should not exist yet"
        );

        mark_component_starting(&component);

        let snapshot = snapshot();
        let entry = snapshot
            .components
            .get(&component)
            .expect("component should be present after mark_component_starting");

        assert_eq!(entry.status, "starting");
        assert!(entry.last_ok.is_none());
    }

    #[test]
    fn mark_component_starting_invalidates_a_previous_ok() {
        // The registry outlives a daemon reload, so a replacement incarnation
        // reuses the name its predecessor wrote to. Inheriting that `ok` would
        // report a component as healthy on evidence it never produced.
        let component = unique_component("health-starting-reset");

        mark_component_ok(&component);
        let healthy = snapshot();
        let healthy = healthy
            .components
            .get(&component)
            .expect("component should exist after mark_component_ok");
        assert_eq!(healthy.status, "ok");
        assert!(healthy.last_ok.is_some());

        mark_component_starting(&component);

        let restarted = snapshot();
        let restarted = restarted
            .components
            .get(&component)
            .expect("component should still exist");
        assert_eq!(restarted.status, "starting");
        assert!(
            restarted.last_ok.is_none(),
            "the previous incarnation's last_ok must not carry forward"
        );
    }

    #[test]
    fn mark_component_starting_preserves_a_recorded_error_and_restart_count() {
        // Erasing a recorded failure is how a status becomes misleading:
        // "starting" is not evidence that a prior error has been resolved.
        let component = unique_component("health-starting-keeps-error");

        bump_component_restart(&component);
        mark_component_error(&component, "boom");

        mark_component_starting(&component);

        let snapshot = snapshot();
        let entry = snapshot
            .components
            .get(&component)
            .expect("component should exist");

        assert_eq!(entry.status, "starting");
        assert_eq!(entry.last_error.as_deref(), Some("boom"));
        assert_eq!(entry.restart_count, 1);
    }

    #[test]
    fn snapshot_json_contains_registered_component_fields() {
        let component = unique_component("health-json");

        mark_component_ok(&component);

        let json = snapshot_json();
        let component_json = &json["components"][&component];

        assert_eq!(component_json["status"], "ok");
        assert!(component_json["updated_at"].as_str().is_some());
        assert!(component_json["last_ok"].as_str().is_some());
        assert!(json["uptime_seconds"].as_u64().is_some());
    }
}
