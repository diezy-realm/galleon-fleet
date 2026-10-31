#![allow(clippy::unwrap_used, clippy::expect_used)] // RATCHET(BL-091): 1 per 2026-10-29; jangan menambah.

use std::sync::Arc;

pub mod util;
pub mod whatsapp;
#[cfg(feature = "whatsapp-web")]
pub mod whatsapp_storage;
#[cfg(feature = "whatsapp-web")]
pub mod whatsapp_web;

pub use whatsapp::WhatsAppChannel;
#[cfg(feature = "whatsapp-web")]
pub use whatsapp_storage::RusqliteStore;
#[cfg(feature = "whatsapp-web")]
pub use whatsapp_web::WhatsAppWebChannel;

#[async_trait::async_trait]
pub trait AudioTranscriber: Send + Sync {
    async fn transcribe(&self, audio_data: &[u8], file_name: &str) -> anyhow::Result<String>;
    #[cfg(test)]
    fn bound_provider(&self) -> &str {
        ""
    }
}

#[async_trait::async_trait]
pub trait TextToSpeechSynthesizer: Send + Sync {
    async fn synthesize_opus(&self, text: &str) -> anyhow::Result<Vec<u8>>;
}

#[async_trait::async_trait]
impl TextToSpeechSynthesizer for Arc<dyn TextToSpeechSynthesizer> {
    async fn synthesize_opus(&self, text: &str) -> anyhow::Result<Vec<u8>> {
        (**self).synthesize_opus(text).await
    }
}
