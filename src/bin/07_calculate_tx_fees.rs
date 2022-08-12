/*!
An example of obtaining and calculating the partial fee that a tx will cost.

This example takes 1-3 args;
- an encoded signed extrinsic
- a block number (if not a number, use latest block)
- a URL to query (if not provided, point to localhost)

Note that URLs must be suffixed with a port number. For most public instances if the URL
is WSS (eg those used in polkadot.js) the port will be 443.

Usage example:

```
cargo run --bin 07_calculate_tx_fees -- \
    0x31028400d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d01689fb02938b62400e5b09327e4a3631c5f6877eb504b58b5a769d889562e94165b03d35645363cd2150e33c4ea3b214c003b65160fd8ae0a0c8ac462747b458a450300000500001cbd2d43530a44705ad088af313e18f80b53ef16b36177cd4b77b846f2a5f07ce5c0 \
    none \
    ws://localhost:9944
```

*/

use utils::ws_client;
use std::{env, process};
use jsonrpsee::{core::client::ClientT, rpc_params};
use serde_json::Value;

const LOCAL_URL: &str = "ws://localhost:9944";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args();

    // ignore program name:
    args.next();

    let extrinsic_hex: String = match args.next() {
        Some(hex) => hex,
        None => {
            eprintln!("cargo run --bin 07_calculate_tx_fees -- EXTRINSIC_HEX [BLOCK_NUMBER] [RPC_URL]");
            process::exit(1);
        }
    };

    let block_number: Option<u64> = args.next().and_then(|n| n.parse().ok());
    let rpc_url: String = args.next().unwrap_or(LOCAL_URL.to_string());

    println!();
    println!("Extrinsix hex: {extrinsic_hex}");
    println!("Block number:  {block_number:?}");
    println!("RPC URL:       {rpc_url}");

    let client = ws_client(&rpc_url).await?;

    // First, convert the block number into a block hash:
    let block_hash = client
        .request::<String>("chain_getBlockHash", rpc_params![block_number])
        .await
        .expect("cannot get block hash for the provided block number");

    println!("Block hash:    {block_hash}");

    // Now, pass this into payment_queryFeeDetails:
    let fee_details_value = client
        .request::<Value>("payment_queryFeeDetails", rpc_params![extrinsic_hex.clone(), block_hash.clone()])
        .await
        .expect("cannot get queryFeeDetails back for extrinsic");

    let inclusion_fee = &fee_details_value["inclusionFee"];
    let weight_fee = to_number(&inclusion_fee["adjustedWeightFee"]);
    let base_fee = to_number(&inclusion_fee["baseFee"]);
    let len_fee = to_number(&inclusion_fee["lenFee"]);

    // We can also fetch the underlying estimated weight/partialFee:
    let fee_info_value = client
        .request::<Value>("payment_queryInfo", rpc_params![extrinsic_hex, block_hash])
        .await
        .expect("cannot get queryInfo back for extrinsic");

    let weight = fee_info_value["weight"].as_i64().expect("weight should exist");
    let partial_fee = fee_info_value["partialFee"].as_str().expect("partialFee should exist");

    println!();
    // The cost to include the extrinsic in a block. Takes into account the cost
    // to verify the signature and that sort of thing. A fixed fee regardless of
    // tx details.
    println!("Base fee:            {base_fee}");
    // A fee which is based on the length of the extrinsic; longer extrinsics
    // pay more. I geuss this is how you pay for the storage of the tx.
    println!("Length fee:          {len_fee}");
    // A fee paid based on the weight of the extrinsic (ie how much processing etc
    // does this particular tx cost). This weight is adjusted to take into account
    // network load and such, which is why this is the "adjusted" weight fee.
    // Something like `targeted_fee_adjustedment x weight`.
    println!("Adjusted weight fee: {weight_fee}");
    // The underlying weight of the transaction, I think, before it's adjusted.
    // This represents the cost to process that particular transaction, but doesn't
    // take into account how busy the network is.
    println!("Weight:              {weight}");

    // The partial fee is the total fee paid minus a tip. It's basically the sum
    // of the base fee, length fee and adjusted weight fee.
    println!();
    println!("Partial fee:         {partial_fee}");
    assert_eq!((base_fee + len_fee + weight_fee).to_string(), partial_fee);

    // NOTE: When an extrinsic is submitted, it's actual weight ends up in
    // ExtrinsicSuccess, as does a `paysFee` parameter. The node is free to
    // set each of these to whatever it likes to modify the actual fee paid
    // or refund it entirely with `PaysFee::No`.
    //
    // This, to calculate the actual fee paid on an extrinsic in a block, you
    // must do something like:
    //
    // `fee = len_fee + base_fee + (weight_fee / wight * new_weight_from_ext_success)`
    //
    // Also note; this is only applicable to Polkadot and chains which copy the
    // way that Polkadot does fees. Chains can do whatever they like, really.

    Ok(())
}

fn to_number(value: &Value) -> u128 {
    let s = value.as_str().expect("value should be a hex string");
    let s = &s[2..]; // trim 0x
    u128::from_str_radix(s, 16).expect("value should be valid u128")
}
