// Tor integration for message server
//
// Provides TorManager for creating and managing Tor onion services

use arti_client::TorClient;
use tor_hsservice::config::OnionServiceConfigBuilder;
use tor_hsservice::{HsNickname, StreamRequest, handle_rend_requests};
use tor_rtcompat::PreferredRuntime;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use futures::stream::BoxStream;
use safelog::DisplayRedacted;

/// Tor client wrapper for message server
pub struct TorManager {
    client: Arc<TorClient<PreferredRuntime>>,
    key_dir: PathBuf,
}

impl TorManager {
    /// Create a new Tor manager
    pub async fn new(key_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&key_dir)
            .context("Failed to create key directory")?;

        tracing::info!("Bootstrapping Tor client...");

        let client = TorClient::create_bootstrapped(Default::default())
            .await
            .context("Failed to bootstrap Tor client")?;

        tracing::info!("✓ Tor client bootstrapped successfully");

        Ok(Self {
            client: Arc::new(client),
            key_dir,
        })
    }

    /// Get the Tor client
    pub fn client(&self) -> Arc<TorClient<PreferredRuntime>> {
        self.client.clone()
    }

    /// Create an onion service for a fortress
    pub async fn create_onion_service(
        &self,
        nickname: &str,
    ) -> Result<(String, BoxStream<'static, StreamRequest>)> {
        let key_path = self.key_dir.join(nickname);
        std::fs::create_dir_all(&key_path)?;

        let hs_nickname: HsNickname = nickname.parse()
            .context("Invalid onion service nickname")?;

        let svc_config = OnionServiceConfigBuilder::default()
            .nickname(hs_nickname)
            .build()
            .context("Failed to build onion service config")?;

        tracing::info!("Launching onion service: {}", nickname);

        let (onion_service, request_stream) = self.client
            .launch_onion_service(svc_config)
            .context("Failed to launch onion service")?;

        // Wait for onion address (poll until available)
        tracing::info!("Waiting for onion address...");
        let onion_addr = loop {
            if let Some(addr) = onion_service.onion_address() {
                break addr;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };

        let onion_string = onion_addr.display_unredacted().to_string();
        tracing::info!("✓ Onion service ready: {}", onion_string);

        // Wrap the request stream with handle_rend_requests
        let stream_requests = handle_rend_requests(request_stream);

        Ok((onion_string, Box::pin(stream_requests)))
    }

    /// Connect to an onion service
    pub async fn connect_to_onion(
        &self,
        address: &str,
        port: u16,
    ) -> Result<arti_client::DataStream> {
        tracing::info!("Connecting to onion service: {}:{}", address, port);

        // Remove .onion suffix if present
        let addr = address.trim_end_matches(".onion");

        let stream = self.client
            .connect((addr, port))
            .await
            .context("Failed to connect to onion service")?;

        tracing::info!("✓ Connected to onion service");

        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_tor_manager_creation() {
        let dir = tempdir().unwrap();
        let key_dir = dir.path().join("keys");

        let manager = TorManager::new(key_dir).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires network access and takes time (~30-60 seconds)
    async fn test_onion_service_creation() {
        let dir = tempdir().unwrap();
        let key_dir = dir.path().join("keys");

        let manager = TorManager::new(key_dir).await.unwrap();
        let result = manager.create_onion_service("test-fortress").await;

        assert!(result.is_ok());
        let (address, _stream) = result.unwrap();
        assert!(address.len() > 10);
        tracing::info!("Created onion address: {}", address);
    }
}
