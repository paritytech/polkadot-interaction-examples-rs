use serde_json::{json, Value};

/// Make an RPC request to the localhost node over HTTP.
pub async fn rpc_to_localhost<Params: serde::Serialize>(
    method: &str,
    params: Params,
) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let mut body: Value = client
        .post("http://localhost:9933")
        .json(&json! {{
            // Used to correlate request with response over socket connections.
            // not needed here over our simple HTTP connection, so just set it
            // to 1 always:
            "id": 1,
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }})
        .send()
        .await?
        .json()
        .await?;

    // take the "result" out of the JSONRPC response:
    Ok(body["result"].take())
}

trait RpcParams {
    fn into_params(self) -> Value;
}
