/*!
We can get the metadata for a chain, which describes, for each module (pallet) that exists:

- The various calls that you can make to interact with it (the "extrinsics", for example
  a call maye exists to transfer balance between accounts),
- Consts that are relevant to the module (for the balances module, we may have an
  ExistentialDeposit value for instance; the minimum deposit that can exist in an account).
- Events that can be emitted (eg, "a transfer has happened from X to Y with balance B"),
- Storage used by the module (eg the balances module keeps a hash of account ID to balance info).

The calls (extrinsics) in particular are interesting, and can be submitted to the chain using
the RPC method (that can be discovered in the 01_basic example) "author_submitExtrinsic".

The Polkadot JS API uses the metadata to generate its structure; see
https://polkadot.js.org/docs/api/start/basics

Note that at the time of writing, where we get back "V13" metadata, we can see information about the extrinsics
and such available, but the named types aren't super useful, except to give you a starting point to
then dig into polkadot/primitives (and then the more foundational substrate primitives in the substrate
repo (see example 02).

```
cargo run --bin 03_metadata
```
*/

use frame_metadata::RuntimeMetadataPrefixed;
use parity_scale_codec::Decode;
use std::borrow::BorrowMut;
use utils::rpc_to_localhost;

#[tokio::main]
async fn main() {
    // Get chain metadata (I'm using a helper function now to make JSONRPC requests and
    // give back the "result"s to save some lines of code..).
    let res = rpc_to_localhost("state_getMetadata", ()).await.unwrap();

    // Decode the hex value into bytes (which are the SCALE encoded metadata details):
    let metadata_hex = res.as_str().unwrap();
    println!("Metadata hex: {:?}", metadata_hex);
    let metadata_bytes = hex::decode(&metadata_hex.trim_start_matches("0x")).unwrap();

    // Fortunately, we know what type the metadata is, so we are able to decode our SCALEd bytes to it:
    let decoded = RuntimeMetadataPrefixed::decode(&mut metadata_bytes.as_slice()).unwrap();

    let metadata = match decoded.1 {
        frame_metadata::RuntimeMetadata::V14(v14) => v14,
        _ => panic!("ops inval metadata"),
    };

    // let mut tys: Vec<_> = metadata.types.types().iter().map(|ty| {
    //     let name = ty.ty().path().segments().join("::");
    //     (name, ty.id(), ty)
    // }).collect();
    // tys.sort_by_key(|(name, ..)| name.clone());
    // for (t, id, _) in tys.iter() {
    //     println!("ID {} {}", id, t);
    // }
    println!("\n");
    // We'll finally re-encode to JSON to make prettier output.
    let output = serde_json::to_string_pretty(&metadata).unwrap();
    println!("{}", output);
}
