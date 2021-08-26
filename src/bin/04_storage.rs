/*!
This is quite heavily inspired by https://www.shawntabrizi.com/substrate/transparent-keys-in-substrate/,
which talks about how to access storage items from a substrate node.

We work through a few examples of plucking data out of substrate storage!

```
cargo run --bin 04_storage
```
*/

use std::convert::TryInto;
use utils::rpc_to_localhost;
use sp_core::{ crypto::{ AccountId32 }, hashing };
use sp_keyring::AccountKeyring;
use parity_scale_codec::{ Decode };
use sp_core::crypto::{ Ss58Codec, Ss58AddressFormat };

#[tokio::main]
async fn main() {

    {
        // We can look in metadata to see what's been stored. We note the "storage prefix"
        // and then "name" of the item we're interested in. First, we'll find out the total
        // number of tokens issued. The following storage prefix and name sound plausible:
        let storage_prefix = "Balances";
        let storage_name = "TotalIssuance";

        // We hash those two values like so:
        let storage_prefix_hashed = hashing::twox_128(storage_prefix.as_bytes());
        let storage_name_hashed = hashing::twox_128(storage_name.as_bytes());

        // We then append them together:
        let mut storage_key = Vec::new();
        storage_key.extend_from_slice(&storage_prefix_hashed);
        storage_key.extend_from_slice(&storage_name_hashed);

        // And we convert them into a hex string:
        let storage_key_hex = format!("0x{}", hex::encode(&storage_key));
        println!("Balances TotalIssuance Hex: {}", storage_key_hex);

        // Finally, we send that hex string to the "state_getStorage" RPC call to query:
        let result_hex = rpc_to_localhost("state_getStorage", (storage_key_hex,)).await.unwrap();

        // The result is a SCALE encoded value. What is the type of the value? Well, according to
        // the metadata, it's a type `T::Balance`. But what's that? Well, essentially, the Balances
        // pallet in `substrate/frame/balances/src/lib.rs` has a type called TotalIssuance, which a
        // `#[pallet::storage]` macro is applied to. This references the type of thing stored;
        // `T::Balance`, where `T` is some `Config` trait defined with a bunch of associated types
        // including `Balance`.
        //
        // Polkadot implements that trait in `polkadot/runtime/polkadot/src/lib.rs` for its own
        // Runtime, and there we can see that balance is instantiated to be a `u128`. So, let's
        // decode the SCALE encoded value we get back into a u128 to see the value (first decoding
        // the hex string to bytes):
        let result_hex_str = result_hex.as_str().unwrap();
        let result_bytes = hex::decode(result_hex_str.trim_start_matches("0x")).unwrap();
        let total_issued = u128::decode(&mut result_bytes.as_slice()).unwrap();

        println!("Total issued encoded response: {}", result_hex_str);
        println!("Total issued: {}", total_issued);
    }

    {
        // Similar to the above example, and to sanity check the below one, we can also get hold of all keys
        // in the system (following https://www.shawntabrizi.com/substrate/transparent-keys-in-substrate/):
        let storage_prefix = "System";
        let storage_name = "Account";

        // We hash those two values like so:
        let storage_prefix_hashed = hashing::twox_128(storage_prefix.as_bytes());
        let storage_name_hashed = hashing::twox_128(storage_name.as_bytes());

        // We then append them together and hex them:
        let mut storage_key = Vec::new();
        storage_key.extend_from_slice(&storage_prefix_hashed);
        storage_key.extend_from_slice(&storage_name_hashed);
        let storage_key_hex = format!("0x{}", hex::encode(&storage_key));

        let results = rpc_to_localhost("state_getKeys", (storage_key_hex,)).await.unwrap();
        let result_vec: Vec<Vec<u8>> = results
            .as_array().unwrap()
            .into_iter()
            .map(|json| json.as_str().unwrap())
            .map(|hex| hex::decode(hex.trim_start_matches("0x")).unwrap())
            .collect();

        // because account IDs are hashed using Blake2_128Concat (which basically means, run
        // black128 hash on the bytes and then concat the raw value to the end), we know that
        // the last 32 bytes of each thing is an account ID, so we can chop those off to list the
        // accounts (we ss58 encode so that they match what you see in the UI for polkadot).
        println!("\nList of addresses known to system:");
        for res in result_vec {
            let last32 = &res[res.len() - 32 ..];
            let last32_arr: [u8; 32] = last32.try_into().unwrap();
            let address: AccountId32 = last32_arr.into();

            // The address you see is basically the account ID + a version (ie "this is a polkadot address")
            // encoded into SS58 format (see https://github.com/paritytech/substrate/wiki/External-Address-Format-(SS58)):
            println!("{}", address.to_ss58check_with_version(Ss58AddressFormat::PolkadotAccount));
        }
    }

    {
        // We can go one step further and get the balance for a single account. balances are stored in
        // a type that looks like this in the metadata (run example 03 to see this metadata):
        //
        // ```
        // "Map": {
        //     "hasher": "Blake2_128Concat",
        //     "key": "T::AccountId",
        //     "value": "AccountData<T::Balance>",
        //     "unused": false
        // }
        // ```
        //
        // So, not only do we need the module and item name (System and Account, from the Metadata),
        // but we also need to append the AccountId key we're looking for. (There's a similar looking
        // map in the Balances/Account pallet, but apparently balances are stored in the System pallet;
        // not sure what the deal is there).
        let storage_prefix = "System";
        let storage_name = "Account";

        // This is the concrete type AccountId32, and if we follow the types again
        // in the polkadot implementation of the Balances Config, we can see that
        // this is what a `T::AccountId` is in Polkadot, so we're golden.
        //
        // FYI, if we see an address in the UI, we can convert from that SS568 encoding
        // into an AccountId by running `AccountId32::from_ss58check("the-address").unwrap();`.
        let bobs_account_id = AccountKeyring::Bob.to_account_id();

        // if we like, we can print out Bobs address. This is basically the public address
        // + a version (ie "this is a polkadot address") encoded into SS58 format (see
        // https://github.com/paritytech/substrate/wiki/External-Address-Format-(SS58)):
        println!("\nBobs address: {}", bobs_account_id.to_ss58check_with_version(Ss58AddressFormat::PolkadotAccount));

        // Hash things:
        let storage_prefix_hashed = hashing::twox_128(storage_prefix.as_bytes());
        let storage_name_hashed = hashing::twox_128(storage_name.as_bytes());
        let bobs_account_id_hashed = hashing::blake2_128(bobs_account_id.as_ref());

        // Blake2_128Concat means: Run blake2_128 to hash the key, and then concat the
        // raw value at the end, so we'll do that when building up our storage request
        // (see https://www.shawntabrizi.com/substrate/transparent-keys-in-substrate/).

        // Concat, appending Bob's address bytes to the end for the blake2_128concat:
        let mut storage_key = Vec::new();
        storage_key.extend_from_slice(&storage_prefix_hashed);
        storage_key.extend_from_slice(&storage_name_hashed);
        storage_key.extend_from_slice(&bobs_account_id_hashed);
        storage_key.extend_from_slice(bobs_account_id.as_ref());

        // hexify the above bytes and make the request to get the value back:
        let storage_key_hex = format!("0x{}", hex::encode(&storage_key));
        println!("AccountId storage key hex: {}", storage_key_hex);

        let result_hex = rpc_to_localhost("state_getStorage", (storage_key_hex,)).await.unwrap();
        let result_scaled = hex::decode(result_hex.as_str().unwrap().trim_start_matches("0x")).unwrap();

        // If we look at how account data is stored, we find it's stored in the type
        // `AccountInfo<T::Index, T::AccountData>`. Remembering that `T` here will be the polkadot runtime,
        // we can see that in `polkadot/runtime/src.lib.rs`:
        //
        // ```
        // type Index = Nonce;
        // type AccountData = pallet_balances::AccountData<Balance>;
        // ```
        //
        // Well, `Nonce` is just an alias for u32, and Balance is just an alias for `u128`, so we end up
        // wanting to decode our result into this type to read it:
        type PolkadotAccountInfo = pallet_system::AccountInfo<u32, pallet_balances::AccountData<u128>>;
        let account_info = PolkadotAccountInfo::decode(&mut result_scaled.as_ref());
        println!("{:?}", account_info);

    }
}
