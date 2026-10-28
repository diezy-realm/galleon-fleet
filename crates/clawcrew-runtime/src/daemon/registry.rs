use anyhow::Result;
use clawcrew_config::schema::{Config, MqttConfig};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use super::{DaemonTransportContext, GatewayReadinessReporter, SocketReadinessReporter};
use crate::tui_identity::TuiRegistry;

pub type StarterFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

pub use galleon_infra::gateway_controls::GatewayReloadControls;

pub type GatewayStarter = Box<
    dyn Fn(
            String,
            u16,
            Config,
            Option<broadcast::Sender<Value>>,
            Option<GatewayReloadControls>,
            Option<Arc<TuiRegistry>>,
            Option<GatewayReadinessReporter>,
        ) -> StarterFuture
        + Send
        + Sync,
>;

/// Starts the supervised channel orchestrator for one daemon run/reload iteration.
pub type ChannelsStarter = Box<dyn Fn(Config, CancellationToken) -> StarterFuture + Send + Sync>;

/// Starts the local IPC transport and optionally reports its secured bind.
pub type SocketStarter = Box<
    dyn Fn(
            Arc<DaemonTransportContext>,
            CancellationToken,
            Arc<AtomicUsize>,
            Option<SocketReadinessReporter>,
        ) -> StarterFuture
        + Send
        + Sync,
>;

/// Starts a supervised background transport using the shared daemon transport context.
pub type TransportStarter = Box<
    dyn Fn(Arc<DaemonTransportContext>, CancellationToken, Arc<AtomicUsize>) -> StarterFuture
        + Send
        + Sync,
>;

/// Starts the MQTT SOP listener for one configured MQTT channel alias.
pub type MqttStarter = Box<dyn Fn(MqttConfig) -> StarterFuture + Send + Sync>;

#[derive(Default)]
pub struct DaemonRegistry {
    gateway_start: Option<GatewayStarter>,
    channels_start: Option<ChannelsStarter>,
    socket_start: Option<SocketStarter>,
    relay_start: Option<TransportStarter>,
    enroll_start: Option<TransportStarter>,
    mqtt_start: Option<MqttStarter>,
}

impl DaemonRegistry {
    /// Create an empty registry. Missing starters are treated as unwired
    /// optional subsystems by `daemon::run`.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_gateway(&mut self, starter: GatewayStarter) -> &mut Self {
        self.gateway_start = Some(starter);
        self
    }

    #[cfg(test)]
    fn has_gateway_start(&self) -> bool {
        self.gateway_start.is_some()
    }

    pub fn register_channels(&mut self, starter: ChannelsStarter) -> &mut Self {
        self.channels_start = Some(starter);
        self
    }

    #[cfg(test)]
    fn has_channels_start(&self) -> bool {
        self.channels_start.is_some()
    }

    pub fn register_socket(&mut self, starter: SocketStarter) -> &mut Self {
        self.socket_start = Some(starter);
        self
    }

    #[cfg(test)]
    pub(crate) fn has_socket_start(&self) -> bool {
        self.socket_start.is_some()
    }

    pub fn register_relay(&mut self, starter: TransportStarter) -> &mut Self {
        self.relay_start = Some(starter);
        self
    }

    pub(crate) fn take_relay_start(&mut self) -> Option<TransportStarter> {
        self.relay_start.take()
    }

    /// Register the certificate enrollment endpoint (the bootstrap surface a
    /// certless client reaches for its first cert). Supervised like the relay
    /// bridge; the starter parks when `[enroll]` is disabled.
    pub fn register_enroll(&mut self, starter: TransportStarter) -> &mut Self {
        self.enroll_start = Some(starter);
        self
    }

    pub(crate) fn take_enroll_start(&mut self) -> Option<TransportStarter> {
        self.enroll_start.take()
    }

    pub fn register_mqtt(&mut self, starter: MqttStarter) -> &mut Self {
        self.mqtt_start = Some(starter);
        self
    }

    #[cfg(test)]
    fn has_mqtt_start(&self) -> bool {
        self.mqtt_start.is_some()
    }

    pub(crate) fn take_gateway_start(&mut self) -> Option<GatewayStarter> {
        self.gateway_start.take()
    }

    pub(crate) fn take_channels_start(&mut self) -> Option<ChannelsStarter> {
        self.channels_start.take()
    }

    pub(crate) fn take_socket_start(&mut self) -> Option<SocketStarter> {
        self.socket_start.take()
    }

    pub(crate) fn take_mqtt_start(&mut self) -> Option<MqttStarter> {
        self.mqtt_start.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gateway_starter() -> GatewayStarter {
        Box::new(|_, _, _, _, _, _, _| Box::pin(async { Ok(()) }))
    }

    fn channels_starter() -> ChannelsStarter {
        Box::new(|_, _| Box::pin(async { Ok(()) }))
    }

    fn socket_starter() -> SocketStarter {
        Box::new(|_, _, _, _| Box::pin(async { Ok(()) }))
    }

    fn transport_starter() -> TransportStarter {
        Box::new(|_, _, _| Box::pin(async { Ok(()) }))
    }

    fn mqtt_starter() -> MqttStarter {
        Box::new(|_| Box::pin(async { Ok(()) }))
    }

    #[test]
    fn new_registry_has_no_start_hooks() {
        let registry = DaemonRegistry::new();

        assert!(!registry.has_gateway_start());
        assert!(!registry.has_channels_start());
        assert!(!registry.has_socket_start());
        assert!(!registry.has_mqtt_start());
    }

    #[test]
    fn builder_records_typed_start_hooks() {
        let mut registry = DaemonRegistry::new();
        registry
            .register_gateway(gateway_starter())
            .register_channels(channels_starter())
            .register_socket(socket_starter())
            .register_relay(transport_starter())
            .register_mqtt(mqtt_starter());

        assert!(registry.has_gateway_start());
        assert!(registry.has_channels_start());
        assert!(registry.has_socket_start());
        assert!(registry.has_mqtt_start());
    }

    #[test]
    fn taking_start_hooks_consumes_slots() {
        let mut registry = DaemonRegistry::new();
        registry
            .register_gateway(gateway_starter())
            .register_channels(channels_starter())
            .register_socket(socket_starter())
            .register_relay(transport_starter())
            .register_mqtt(mqtt_starter());

        assert!(registry.take_gateway_start().is_some());
        assert!(registry.take_channels_start().is_some());
        assert!(registry.take_socket_start().is_some());
        assert!(registry.take_relay_start().is_some());
        assert!(registry.take_mqtt_start().is_some());

        assert!(!registry.has_gateway_start());
        assert!(!registry.has_channels_start());
        assert!(!registry.has_socket_start());
        assert!(!registry.has_mqtt_start());
    }
}
