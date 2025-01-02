use std::sync::Arc;

use alloy::primitives::Address;
use alloy::pubsub::PubSubFrontend;
use alloy::rpc::types::{Filter, Log};
use alloy::transports::Transport;
use alloy_network::AnyNetwork;
use alloy_provider::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy_rpc_client::ClientBuilder;
use alloy_transport_http::{Client, Http};
use anyhow::Result;

pub fn get_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|err| panic!("Missing env; key={key}; err={err}"))
}

pub fn get_http_provider(endpoint: &str) -> RootProvider<Http<Client>, AnyNetwork> {
    ProviderBuilder::new()
        .network::<AnyNetwork>()
        .on_client(ClientBuilder::default().http(endpoint.parse().unwrap()))
}

pub async fn get_ws_provider(endpoint: &str) -> RootProvider<PubSubFrontend> {
    ProviderBuilder::new()
        .on_ws(WsConnect::new(endpoint))
        .await
        .unwrap()
}

pub fn get_block_range(from_block: u64, to_block: u64, chunk: u64) -> Vec<(u64, u64)> {
    (from_block..=to_block)
        .step_by(chunk as usize)
        .map(|start| (start, (start + chunk - 1).min(to_block)))
        .collect()
}

pub async fn get_logs<P, T>(
    provider: Arc<P>,
    from_block: u64,
    to_block: u64,
    address: Option<Address>,
    events: &[&str],
) -> Result<Vec<Log>>
where
    P: Provider<T> + ?Sized + Send + Sync + 'static,
    T: Transport + Clone + Send + Sync + 'static,
{
    let mut event_filter = Filter::new()
        .from_block(from_block)
        .to_block(to_block)
        .events(events);

    if let Some(address) = address {
        event_filter = event_filter.address(address);
    }

    let logs = provider.get_logs(&event_filter).await?;
    Ok(logs)
}
