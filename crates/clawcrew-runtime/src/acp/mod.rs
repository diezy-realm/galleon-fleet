//! ACP (Agent Control Protocol) server and back-channel implementation.

pub mod channel;
pub mod embedded;
pub mod server;

pub use channel::AcpChannel;
pub use server::{AcpServer, AcpServerConfig};
