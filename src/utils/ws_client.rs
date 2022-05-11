use jsonrpsee::async_client::Client;

/// Build an WebServer client for interacting with the node's RPC.
pub async fn ws_client(url: &str) -> anyhow::Result<Client> {
    let url: jsonrpsee::client_transport::ws::Uri = url.parse()?;

    let (sender, receiver) = jsonrpsee::client_transport::ws::WsTransportClientBuilder::default()
        .build(url)
        .await?;

    Ok(jsonrpsee::core::client::ClientBuilder::default()
        .max_notifs_per_subscription(4096)
        .build_with_tokio(sender, receiver))
}
