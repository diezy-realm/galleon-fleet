use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Clone)]
pub struct GatewayReloadControls {
    pub shutdown_tx: watch::Sender<bool>,
    pub reload_tx: watch::Sender<bool>,
}

#[derive(Clone)]
pub struct GatewayReadinessReporter(pub Arc<dyn Fn(SocketAddr) + Send + Sync>);

impl GatewayReadinessReporter {
    pub fn new(report: impl Fn(SocketAddr) + Send + Sync + 'static) -> Self {
        Self(Arc::new(report))
    }

    pub fn report_ready(&self, addr: SocketAddr) {
        (self.0)(addr);
    }
}
