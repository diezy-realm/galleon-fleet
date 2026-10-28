pub mod log;
pub mod multi;
pub mod noop;
#[cfg(feature = "observability-otel")]
pub mod otel;
#[cfg(feature = "observability-otel")]
pub mod otel_config;
#[cfg(feature = "observability-prometheus")]
pub mod prometheus;
pub mod runtime_trace;
pub mod traits;
pub mod verbose;

#[allow(unused_imports)]
pub use self::log::LogObserver;
#[allow(unused_imports)]
pub use self::multi::MultiObserver;
#[cfg(feature = "observability-otel")]
use self::otel_config::OtelContentConfig;
pub use noop::NoopObserver;
#[cfg(feature = "observability-otel")]
pub use otel::OtelObserver;
#[cfg(feature = "observability-prometheus")]
pub use prometheus::PrometheusObserver;
pub use traits::{Observer, ObserverEvent, ObserverMetric};
#[allow(unused_imports)]
pub use verbose::VerboseObserver;

use std::sync::Arc;

use galleon_config::schema::{ObservabilityBackend, ObservabilityConfig};

/// Drop-safe lifecycle bracket for one logical agent turn.
///
/// Construction emits exactly one [`ObserverEvent::AgentStart`]. Calling
pub use clawcrew_api::observability_traits::AgentTurnGuard;

/// Re-export process-wide broadcast hook and TeeObserver from the leaf SSOT (`clawcrew_api::observability_traits`).
pub use clawcrew_api::observability_traits::{
    BroadcastHookGuard, TeeObserver, clear_broadcast_hook, current_broadcast_hook,
    set_broadcast_hook, set_scoped_broadcast_hook,
};

/// Guard that flushes its observer on drop — the telemetry analogue of
/// [`AgentTurnGuard`]. Held for the lifetime of a short-lived agent
/// invocation (today: the CLI one-shot, `clawcrew agent -m ...`), whose
/// process exits before the OTLP batch exporter / metric
/// `PeriodicReader`'s background interval fires. Without this flush all
/// buffered telemetry — including the never-ended `gen_ai.agent.invoke`
/// span, which is only `.end()`'d inside [`Observer::flush`] — is lost
/// when the runtime is torn down.
///
/// Long-lived callers (daemon heartbeat/cron, channel `process_message`,
/// subagent spawns) pass `interactive = false` and skip this guard: they
/// rely on the periodic export firing on its own cadence, and a flush
/// per turn would add a synchronous OTLP HTTP POST to every invocation.
///
/// Backend-agnostic: calls `Observer::flush()`, which is a no-op for
/// synchronous backends (`Log`/`Verbose`/`Noop`) and meaningless-but-
/// harmless for pull backends (`Prometheus` — see startup warning).
#[must_use = "hold the guard for the lifetime of the agent invocation; dropping it flushes"]
pub struct FlushGuard {
    observer: Arc<dyn Observer>,
    done: bool,
}

impl FlushGuard {
    /// Construct a guard that will flush `observer` when dropped.
    pub fn new(observer: Arc<dyn Observer>) -> Self {
        Self {
            observer,
            done: false,
        }
    }

    /// Flush immediately and mark the guard spent so a later `Drop` is a no-op.
    pub fn fire(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        self.observer.flush();
    }
}

impl Drop for FlushGuard {
    fn drop(&mut self) {
        self.fire();
    }
}

/// Emit startup warnings for any non-`Off` OTel content policy. Behavior is
/// unchanged from the pre-isolation inline block: a non-`Off` GenAI or tool
/// I/O policy surfaces a privacy reminder at observer construction time.
#[cfg(feature = "observability-otel")]
fn warn_otel_content_policy(config: OtelContentConfig) {
    use galleon_config::schema::OtelContentPolicy;

    if config.genai_policy != OtelContentPolicy::Off {
        let msg = match config.genai_policy {
            OtelContentPolicy::Redacted => {
                "otel_genai_content=redacted: OTel GenAI input/output will be captured with sensitive-content processing and per-field truncation. Processed content may still contain information that could lead to leakage. Enable only when necessary."
            }
            OtelContentPolicy::Full => {
                "otel_genai_content=full: OTel GenAI input/output will be captured with sensitive-content processing but WITHOUT truncation. Use only in controlled environments."
            }
            _ => unreachable!(),
        };
        ::galleon_log::record!(
            WARN,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                .with_outcome(::galleon_log::EventOutcome::Unknown),
            msg
        );
    }
    if config.tool_io_policy != OtelContentPolicy::Off {
        let msg = match config.tool_io_policy {
            OtelContentPolicy::Redacted => {
                "otel_tool_io=redacted: OTel tool input/output will be captured with sensitive-content processing and per-field truncation. Processed content may still contain information that could lead to leakage. Enable only when necessary."
            }
            OtelContentPolicy::Full => {
                "otel_tool_io=full: OTel tool input/output will be captured with sensitive-content processing but WITHOUT truncation. Use only in controlled environments."
            }
            _ => unreachable!(),
        };
        ::galleon_log::record!(
            WARN,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                .with_outcome(::galleon_log::EventOutcome::Unknown),
            msg
        );
    }
}

/// Factory: create the right observer from config
pub fn create_observer(config: &ObservabilityConfig) -> Box<dyn Observer> {
    Box::new(TeeObserver::new(create_primary_observer(config)))
}

fn create_primary_observer(config: &ObservabilityConfig) -> Box<dyn Observer> {
    match config.backend {
        ObservabilityBackend::Log => Box::new(LogObserver::new()),
        ObservabilityBackend::Verbose => Box::new(VerboseObserver::new()),
        ObservabilityBackend::Prometheus => {
            #[cfg(feature = "observability-prometheus")]
            {
                Box::new(PrometheusObserver::shared())
            }
            #[cfg(not(feature = "observability-prometheus"))]
            {
                ::galleon_log::record!(
                    WARN,
                    ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                        .with_outcome(::galleon_log::EventOutcome::Unknown),
                    "Prometheus backend requested but this build was compiled without `observability-prometheus`; falling back to noop."
                );
                Box::new(NoopObserver)
            }
        }
        ObservabilityBackend::Otel => {
            #[cfg(feature = "observability-otel")]
            {
                let content_config = OtelContentConfig::from_observability_config(config);

                match OtelObserver::new(
                    config.otel_endpoint.as_deref(),
                    config.otel_service_name.as_deref(),
                    config.otel_headers.clone(),
                    content_config,
                ) {
                    Ok(obs) => {
                        warn_otel_content_policy(content_config);

                        ::galleon_log::record!(
                            INFO,
                            ::galleon_log::Event::new(
                                module_path!(),
                                ::galleon_log::Action::Note
                            )
                            .with_attrs(
                                ::serde_json::json!({"endpoint": config
                                .otel_endpoint
                                .as_deref()
                                .unwrap_or("http://localhost:4318")})
                            ),
                            "OpenTelemetry observer initialized"
                        );
                        Box::new(obs)
                    }
                    Err(e) => {
                        ::galleon_log::record!(
                            ERROR,
                            ::galleon_log::Event::new(
                                module_path!(),
                                ::galleon_log::Action::Fail
                            )
                            .with_outcome(::galleon_log::EventOutcome::Failure)
                            .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
                            "Failed to create OTel observer. Falling back to noop."
                        );
                        Box::new(NoopObserver)
                    }
                }
            }
            #[cfg(not(feature = "observability-otel"))]
            {
                ::galleon_log::record!(
                    WARN,
                    ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                        .with_outcome(::galleon_log::EventOutcome::Unknown),
                    "OpenTelemetry backend requested but this build was compiled without `observability-otel`; falling back to noop."
                );
                Box::new(NoopObserver)
            }
        }
        ObservabilityBackend::None => Box::new(NoopObserver),
    }
}

/// Serializes tests (in this module and elsewhere in the crate, e.g. the
/// `agent::loop_` lifecycle-bracket tests that drive `run()` end to end)
/// that install the process-wide broadcast hook, so concurrent test runs
/// don't observe each other's installations. A `tokio::sync::Mutex` (not
/// `parking_lot`) so async tests can hold the guard across the `.await` on
/// `run()` without tripping `clippy::await_holding_lock`; plain `#[test]`s
/// use [`tokio::sync::Mutex::blocking_lock`].
#[cfg(test)]
pub(crate) static HOOK_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    #[test]
    fn factory_none_returns_noop() {
        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::None,
            ..ObservabilityConfig::default()
        };
        assert_eq!(create_observer(&cfg).name(), "noop");
    }

    #[test]
    fn factory_noop_returns_noop() {
        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::None,
            ..ObservabilityConfig::default()
        };
        assert_eq!(create_observer(&cfg).name(), "noop");
    }

    #[test]
    fn factory_log_returns_log() {
        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::Log,
            ..ObservabilityConfig::default()
        };
        assert_eq!(create_observer(&cfg).name(), "log");
    }

    #[test]
    fn factory_verbose_returns_verbose() {
        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::Verbose,
            ..ObservabilityConfig::default()
        };
        assert_eq!(create_observer(&cfg).name(), "verbose");
    }

    #[test]
    fn factory_prometheus_returns_prometheus() {
        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::Prometheus,
            ..ObservabilityConfig::default()
        };
        let expected = if cfg!(feature = "observability-prometheus") {
            "prometheus"
        } else {
            "noop"
        };
        assert_eq!(create_observer(&cfg).name(), expected);
    }

    #[test]
    fn factory_otel_returns_otel() {
        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::Otel,
            otel_endpoint: Some("http://127.0.0.1:19999".into()),
            otel_service_name: Some("test".into()),
            ..ObservabilityConfig::default()
        };
        let expected = if cfg!(feature = "observability-otel") {
            "otel"
        } else {
            "noop"
        };
        assert_eq!(create_observer(&cfg).name(), expected);
    }

    #[test]
    fn factory_opentelemetry_alias() {
        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::Otel,
            otel_endpoint: Some("http://127.0.0.1:19999".into()),
            otel_service_name: Some("test".into()),
            ..ObservabilityConfig::default()
        };
        let expected = if cfg!(feature = "observability-otel") {
            "otel"
        } else {
            "noop"
        };
        assert_eq!(create_observer(&cfg).name(), expected);
    }

    #[test]
    fn factory_otlp_alias() {
        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::Otel,
            otel_endpoint: Some("http://127.0.0.1:19999".into()),
            otel_service_name: Some("test".into()),
            ..ObservabilityConfig::default()
        };
        let expected = if cfg!(feature = "observability-otel") {
            "otel"
        } else {
            "noop"
        };
        assert_eq!(create_observer(&cfg).name(), expected);
    }

    #[test]
    fn unknown_backend_falls_back_to_noop_at_load() {
        let bad: ObservabilityConfig = toml::from_str("backend = \"xyzzy_unknown\"").unwrap();
        assert_eq!(bad.backend, ObservabilityBackend::None);
        let empty: ObservabilityConfig = toml::from_str("backend = \"\"").unwrap();
        assert_eq!(empty.backend, ObservabilityBackend::None);
        assert_eq!(create_observer(&bad).name(), "noop");
    }

    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test observer that counts events, metrics, and flushes, used to
    /// verify the broadcast hook fan-out, that downcasts pass through
    /// `TeeObserver`, and that `FlushGuard` drives `Observer::flush`.
    #[derive(Default)]
    struct CountingObserver {
        events: AtomicUsize,
        metrics: AtomicUsize,
        flushes: AtomicUsize,
    }

    impl Observer for CountingObserver {
        fn record_event(&self, _event: &ObserverEvent) {
            self.events.fetch_add(1, Ordering::SeqCst);
        }

        fn record_metric(&self, _metric: &ObserverMetric) {
            self.metrics.fetch_add(1, Ordering::SeqCst);
        }

        fn flush(&self) {
            self.flushes.fetch_add(1, Ordering::SeqCst);
        }

        fn name(&self) -> &str {
            "counting"
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn broadcast_hook_receives_events_from_factory_observer() {
        let _guard = HOOK_TEST_LOCK.blocking_lock();
        clear_broadcast_hook();

        let hook = Arc::new(CountingObserver::default());
        set_broadcast_hook(hook.clone());

        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::None,
            ..ObservabilityConfig::default()
        };
        let observer = create_observer(&cfg);

        observer.record_event(&ObserverEvent::HeartbeatTick);
        observer.record_event(&ObserverEvent::Error {
            component: "x".into(),
            message: "y".into(),
        });

        assert_eq!(hook.events.load(Ordering::SeqCst), 2);

        clear_broadcast_hook();
    }

    #[test]
    fn broadcast_hook_does_not_receive_metrics() {
        let _guard = HOOK_TEST_LOCK.blocking_lock();
        clear_broadcast_hook();

        let hook = Arc::new(CountingObserver::default());
        set_broadcast_hook(hook.clone());

        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::None,
            ..ObservabilityConfig::default()
        };
        let observer = create_observer(&cfg);

        observer.record_metric(&ObserverMetric::TokensUsed(10));
        observer.record_metric(&ObserverMetric::TokensUsed(20));

        assert_eq!(hook.events.load(Ordering::SeqCst), 0);
        assert_eq!(hook.metrics.load(Ordering::SeqCst), 0);

        clear_broadcast_hook();
    }

    #[test]
    fn broadcast_hook_unset_means_only_primary_runs() {
        let _guard = HOOK_TEST_LOCK.blocking_lock();
        clear_broadcast_hook();

        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::None,
            ..ObservabilityConfig::default()
        };
        let observer = create_observer(&cfg);

        // No hook installed; recording must not panic and must be a no-op.
        observer.record_event(&ObserverEvent::HeartbeatTick);
        observer.record_metric(&ObserverMetric::TokensUsed(1));
    }

    #[test]
    fn scoped_broadcast_hook_guard_clears_installed_hook_on_drop() {
        let _guard = HOOK_TEST_LOCK.blocking_lock();
        clear_broadcast_hook();

        let hook = Arc::new(CountingObserver::default());
        let broadcast_guard = set_scoped_broadcast_hook(hook.clone());

        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::None,
            ..ObservabilityConfig::default()
        };
        let observer = create_observer(&cfg);
        observer.record_event(&ObserverEvent::HeartbeatTick);
        assert_eq!(hook.events.load(Ordering::SeqCst), 1);

        drop(broadcast_guard);
        observer.record_event(&ObserverEvent::HeartbeatTick);
        assert_eq!(hook.events.load(Ordering::SeqCst), 1);

        clear_broadcast_hook();
    }

    #[test]
    fn scoped_broadcast_hook_guard_preserves_replacement_hook() {
        let _guard = HOOK_TEST_LOCK.blocking_lock();
        clear_broadcast_hook();

        let old_hook = Arc::new(CountingObserver::default());
        let old_guard = set_scoped_broadcast_hook(old_hook.clone());

        let new_hook = Arc::new(CountingObserver::default());
        set_broadcast_hook(new_hook.clone());
        drop(old_guard);

        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::None,
            ..ObservabilityConfig::default()
        };
        let observer = create_observer(&cfg);
        observer.record_event(&ObserverEvent::HeartbeatTick);

        assert_eq!(old_hook.events.load(Ordering::SeqCst), 0);
        assert_eq!(new_hook.events.load(Ordering::SeqCst), 1);

        clear_broadcast_hook();
    }

    #[test]
    fn dropping_newer_scoped_broadcast_hook_restores_older_live_hook() {
        let _guard = HOOK_TEST_LOCK.blocking_lock();
        clear_broadcast_hook();

        let old_hook = Arc::new(CountingObserver::default());
        let old_guard = set_scoped_broadcast_hook(old_hook.clone());

        let new_hook = Arc::new(CountingObserver::default());
        let new_guard = set_scoped_broadcast_hook(new_hook.clone());

        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::None,
            ..ObservabilityConfig::default()
        };
        let observer = create_observer(&cfg);
        observer.record_event(&ObserverEvent::HeartbeatTick);
        assert_eq!(old_hook.events.load(Ordering::SeqCst), 0);
        assert_eq!(new_hook.events.load(Ordering::SeqCst), 1);

        drop(new_guard);
        observer.record_event(&ObserverEvent::HeartbeatTick);
        assert_eq!(old_hook.events.load(Ordering::SeqCst), 1);
        assert_eq!(new_hook.events.load(Ordering::SeqCst), 1);

        drop(old_guard);
        observer.record_event(&ObserverEvent::HeartbeatTick);
        assert_eq!(old_hook.events.load(Ordering::SeqCst), 1);
        assert_eq!(new_hook.events.load(Ordering::SeqCst), 1);

        clear_broadcast_hook();
    }

    #[test]
    fn factory_observer_downcasts_through_tee() {
        let _guard = HOOK_TEST_LOCK.blocking_lock();
        clear_broadcast_hook();

        let cfg = ObservabilityConfig {
            backend: ObservabilityBackend::Log,
            ..ObservabilityConfig::default()
        };
        let observer = create_observer(&cfg);

        // `as_any` must surface the primary observer so existing downcasts
        // (e.g. PrometheusObserver in /metrics) keep working through the tee.
        assert!(observer.as_any().downcast_ref::<LogObserver>().is_some());
    }

    #[test]
    fn flush_guard_flushes_on_drop() {
        let observer = Arc::new(CountingObserver::default());
        let guard = FlushGuard::new(observer.clone());
        assert_eq!(observer.flushes.load(Ordering::SeqCst), 0);
        drop(guard);
        assert_eq!(observer.flushes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn flush_guard_fire_is_idempotent() {
        let observer = Arc::new(CountingObserver::default());
        let mut guard = FlushGuard::new(observer.clone());
        guard.fire();
        assert_eq!(observer.flushes.load(Ordering::SeqCst), 1);
        // Second explicit fire is a no-op.
        guard.fire();
        assert_eq!(observer.flushes.load(Ordering::SeqCst), 1);
        // Dropping after fire must not flush again.
        drop(guard);
        assert_eq!(observer.flushes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn agent_turn_guard_closes_once_during_panic_unwind() {
        let observer = CountingObserver::default();
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = AgentTurnGuard::start(
                &observer,
                "provider",
                "model",
                Some("channel".into()),
                Some("agent".into()),
                Some("turn".into()),
            );
            panic!("test unwind");
        }));

        assert!(unwind.is_err());
        assert_eq!(
            observer.events.load(Ordering::SeqCst),
            2,
            "panic unwind must emit one start and one end"
        );
    }

    #[test]
    fn agent_turn_guard_explicit_finish_is_idempotent() {
        let observer = CountingObserver::default();
        let mut guard = AgentTurnGuard::start(
            &observer,
            "provider",
            "model",
            Some("channel".into()),
            Some("agent".into()),
            Some("turn".into()),
        );
        guard.finish();
        guard.finish();
        drop(guard);

        assert_eq!(
            observer.events.load(Ordering::SeqCst),
            2,
            "explicit finish and drop must still emit one matched pair"
        );
    }
}
