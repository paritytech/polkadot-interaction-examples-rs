/*!
This example iterates over the storage keys of the XcmPallet's VersionNotifiers,
which is a double storage map.

The example interprets the storage keys at the byte level and provides the
output of the given keys.

```
cargo run --bin 06_storage_iter
```
 */

use jsonrpsee::{core::client::ClientT, rpc_params};
use parity_scale_codec::Decode;
use sp_core::{hashing, storage::StorageKey};
use utils::ws_client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ws_client("ws://localhost:9944").await?;

    // The VersionNotifiers type of the XcmPallet is defined as:
    //
    // ```
    //  All locations that we have requested version notifications from.
    // 	#[pallet::storage]
    // 	pub(super) type VersionNotifiers<T: Config> = StorageDoubleMap<
    // 		_,
    // 		Twox64Concat,
    // 		XcmVersion,
    // 		Blake2_128Concat,
    // 		VersionedMultiLocation,
    // 		QueryId,
    // 		OptionQuery,
    // 	>;
    // ```

    // Construct the XcmPallet VersionNotifiers key represented by:
    // twox_128(XcmPallet) ++ twox_128(VersionNotifiers)
    let storage_prefix = "XcmPallet";
    let storage_name = "VersionNotifiers";

    // Obtain the twox_128 of the provided values.
    let storage_prefix_hashed = hashing::twox_128(storage_prefix.as_bytes());
    let storage_name_hashed = hashing::twox_128(storage_name.as_bytes());

    // Concat the values.
    let mut storage_key = Vec::new();
    storage_key.extend_from_slice(&storage_prefix_hashed);
    storage_key.extend_from_slice(&storage_name_hashed);

    // Hexify the above bytes and make the request to get the value back:
    let storage_key_hex = format!("0x{}", hex::encode(&storage_key));
    println!("VersionNotifiers storage key: {}", storage_key_hex);

    let params = rpc_params![storage_key_hex.clone()];
    let keys: Vec<StorageKey> = client.request("state_getKeys", params).await?;

    println!("Obtained keys:");
    for key in keys.iter() {
        let key_hex = format!("0x{}", hex::encode(&key));
        println!("Key: {}", key_hex);

        // Obtain the byte representation of the key.
        let key_bytes = &key.0;
        // The first 16 bytes of the key represent the `storage_prefix`.
        let inspected_bytes = &key_bytes[0..16];
        println!(
            "     bytes[ 0..16]: {} == twox_128(\"XcmPallet\")",
            format!("0x{}", hex::encode(&inspected_bytes))
        );
        // The next 16 bytes represent the `storage_name_hashed`.
        let inspected_bytes = &key_bytes[16..32];
        println!(
            "     bytes[16..32]: {} == twox_128(\"VersionNotifiers\")",
            format!("0x{}", hex::encode(&inspected_bytes))
        );

        // The next 8 bytes are represented by the twox64(first key), the hashing produces 8 bytes.
        let inspected_bytes = &key_bytes[32..40];
        println!(
            "     bytes[32..40]: {}                 == twox_64(first key)",
            format!("0x{}", hex::encode(&inspected_bytes))
        );

        // The next 4 bytes represent the first key - `xcm::Version` which is an u32.
        let inspected_bytes = &key_bytes[40..44];
        let xcm_version = xcm::Version::decode(&mut inspected_bytes.clone())?;
        println!(
            "     bytes[40..44]: {}                         == first key - XcmVersion: {}",
            format!("0x{}", hex::encode(&inspected_bytes)),
            xcm_version
        );

        // The next 16 bytes represent the blake2_128(second key), the hashing produces 16 bytes.
        let inspected_bytes = &key_bytes[44..60];
        println!(
            "     bytes[44..60]: {} == blake2_128(second key)",
            format!("0x{}", hex::encode(&inspected_bytes))
        );

        // The remaining bytes are represented by the second key.
        let inspected_bytes = &key_bytes[60..];
        let versioned_multilocation =
            xcm::VersionedMultiLocation::decode(&mut inspected_bytes.clone())?;
        println!(
            "     bytes[60..  ]: {}                     == second key - VersionedMultiLocation: {:?}",
            format!("0x{}", hex::encode(&inspected_bytes)),
            versioned_multilocation
        );

        // Get the value of the storage key.
        let params = rpc_params![key];
        let result_hex: String = client.request("state_getStorage", params).await?;
        let result_bytes = hex::decode(result_hex.trim_start_matches("0x"))?;
        let query_id = xcm::v2::QueryId::decode(&mut result_bytes.as_slice())?;
        println!("  Value: {}\n", query_id);
    }

    Ok(())
}
