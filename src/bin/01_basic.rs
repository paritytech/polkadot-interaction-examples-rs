/*!
The simplest request we can make to a node. We ask for a list of available RPC methods using
the JSONRPC format over a standard HTTP connection (we can by default use http over 9933
or WS over 9944).

Nothing needs signing, and nothing is SCALE encoded.

```
cargo run --bin 01_basic
```
*/

use serde_json::{json, Value};

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();

    // See https://www.jsonrpc.org/specification for more information on
    // the JSON RPC 2.0 format that we use here to talk to nodes.
    let res = client
        .post("http://localhost:9933")
        .json(&json! {{
            "id": 1,
            "jsonrpc": "2.0",
            "method": "rpc_methods"
        }})
        .send()
        .await
        .unwrap();

    let body: Value = res.json().await.unwrap();
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
}
