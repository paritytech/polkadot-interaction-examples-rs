/*!
Fetch some details about the latest block on the chain so far using an RPC method

```
cargo run --bin 02_latest_block
```
*/

use serde_json::{ json, Value };

#[tokio::main]
async fn main() {
    // find the hash of the latest block, so that we can query for
    // details about the block using it.
    let client = reqwest::Client::new();
    let res = client.post("http://localhost:9933")
        .json(&json!{{
            "id": 1,
            "jsonrpc": "2.0",
            // Get the hash of the latest block:
            "method": "chain_getHead"
        }})
        .send()
        .await
        .unwrap();

    let body: Value = res.json().await.unwrap();
    let block_hash = body["result"].as_str().unwrap();
    println!("Latest block hash: {}", block_hash);

    // Get some details, passing the hash we obtained above as a parameter
    // to the JSON RPC call.
    let res = client.post("http://localhost:9933")
        .json(&json!{{
            "id": 1,
            "jsonrpc": "2.0",
            // Get details for the latest block:
            "method": "chain_getBlock",
            "params": [block_hash]
        }})
        .send()
        .await
        .unwrap();

    // We can see the parent hash (prev block hash) and such, but things like "logs"
    // are just a hex string which is SCALE encoded.
    let body: Value = res.json().await.unwrap();
    println!("{}", serde_json::to_string_pretty(&body).unwrap());

    // How do we know what type the scale encoded "logs" should decode to? I thought I'd
    // try to find that out, and ended up following these steps:
    //
    // - Go to the polkadot repo, "primitives" folder.
    // - Spot the `pub type Block = generic::Block<Header, UncheckedExtrinsic>;` line.
    //   I'd guess we are looking at block data, since it's named "block" in the JSON.
    // - See that that uses the `generic::Header<BlockNumber, BlakeTwo256>` type
    // - Follow that to the subtrate repo, primitives/runtime (sp-runtime) folder.
    // - Follow the code to work out that each log entry is a `DigestHash` and the generic
    //   param of `DigestHash` resolves to `sp_core::H256`.
    //
    // Certainly not the easiest thing to do to find out how to read that response information!
    // Armed with that type information, here's the code to decode and view:

    // 1. Get array of hex64'd SCALE encoded logs from the above response
    let logs = body["result"]["block"]["header"]["digest"]["logs"].as_array().unwrap();

    for log in logs {
        // 2. Get hex string for each log entry:
        let log_hex = log.as_str().unwrap();

        // 3. Decode from hex string to bytes:
        let log_bytes = hex::decode(&log_hex.trim_start_matches("0x")).unwrap();

        // 4. Decode into the type we've worked out:
        use sp_runtime::DigestItem;
        use sp_core::H256;
        use parity_scale_codec::Decode;
        let log_entry = <DigestItem<H256>>::decode(&mut log_bytes.as_slice()).unwrap();

        // 4. Prettify to JSON to log:
        println!("{:?}", log_entry);
    }

}
