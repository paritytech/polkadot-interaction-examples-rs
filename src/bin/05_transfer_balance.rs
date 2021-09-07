/*!

Run with:

```
cargo run --bin 05_transfer_balance
```

To transfer money from one account to another, we need to construst an extrinsic which ends up
calling the `transfer` method in the Balances pallet (`substrate/frame/balances/src/lib.rs`).
The transfer method signature, from that code, is this:

```no_run
pub fn transfer(
    origin: OriginFor<T>,
    dest: <T::Lookup as StaticLookup>::Source,
    #[pallet::compact] value: T::Balance,
)
```

The Macro applied on the pallet will generate a `Call` enum containing a `transfer` variant with
those parameters (origin is known separately). We can see this by going to `substrate/frame/balances` and running
`cargo doc --open`. It looks something like:

```no_run
pub enum Call<T: Config<I>, I: 'static = ()> {
    transfer(<T::Lookup as StaticLookup>::Source, T::Balance),
    set_balance(<T::Lookup as StaticLookup>::Source, T::Balance, T::Balance),
    force_transfer(<T::Lookup as StaticLookup>::Source, <T::Lookup as StaticLookup>::Source, T::Balance),
    transfer_keep_alive(<T::Lookup as StaticLookup>::Source, T::Balance),
    transfer_all(<T::Lookup as StaticLookup>::Source, bool),
    // some variants omitted
}
```

However, there are more pallets than just Balances, so this `Call` enum is actually just one variant
in an outer enum. For a Polkadot node, we can see this enum by running `cargo doc --open` in
`polkadot/runtime/polkadot` and finding the outer `Call` enum there.

That all said, the runtime gives us back metadata (see 03_metadata) which, as of V14 (which is soon to be
released) will contain all of the information needed to construct the right value to send off without
having to inspect any code.

An extrinsic is composed of the `Call` like data (to say "I want to call this function with these params",
but is also prepended with either a `u8: 0` value if it's not signed, or signature information if it is.

To construct an extrinsic, we basically need to encode the correct series of bytes to send off. We can do
that using the `Call` enums and such, or we can look at the metadata and manually piece values together that
will encode to the same format at the actual types. We'll mostly do the latter.

So, step 1. What types does the call need? Well, with V14 metadata we'll se able to see the type info in
our metadata bundle. Using some example typed metadata from `https://gist.github.com/ascjones/b76a5345930776ede61dd0f797792ed4`,
let's see what we can find out about the call we're about to make..

First, the "transfer" call in the balances module looks a bit like:

```
{
    //...
    "calls": {
    "ty": 144,
    "calls": [
        {
        "name": "transfer",
        "arguments": [
            {
            "name": "dest",
            "ty": 145
            },
            {
            "name": "value",
            "ty": 68
            }
        ],
        "documentation": []
        },
        //...
}
```

We can look up type #144 in the metadata to see what that looks like (I'll use `jq` to explore it, so we
can view type #144 with `jq '.[1].V14.types.types[144]' ~/the-metadata.json`):

```
{
    "path": [
        "pallet_balances",
        "pallet",
        "Call"
    ],
    "params": [
        {
            "name": "T",
            "type": null
        },
        {
            "name": "I",
            "type": null
        }
    ],
    "def": {
        "variant": {
            "variants": [
                {
                    "name": "transfer",
                    "fields": [
                        {
                            "type": 145,
                            "typeName": "<T::Lookup as StaticLookup>::Source"
                        },
                        {
                            "type": 68,
                            "typeName": "T::Balance"
                        }
                    ]
                },
                {
                    "name": "set_balance",
                    "fields": [
                        {
                            "type": 145,
                            "typeName": "<T::Lookup as StaticLookup>::Source"
                        },
                        {
                            "type": 68,
                            "typeName": "T::Balance"
                        },
                        {
                            "type": 68,
                            "typeName": "T::Balance"
                        }
                    ]
                },
                {
                    "name": "force_transfer",
                    "fields": [
                        {
                            "type": 145,
                            "typeName": "<T::Lookup as StaticLookup>::Source"
                        },
                        {
                            "type": 145,
                            "typeName": "<T::Lookup as StaticLookup>::Source"
                        },
                        {
                            "type": 68,
                            "typeName": "T::Balance"
                        }
                    ]
                },
                {
                    "name": "transfer_keep_alive",
                    "fields": [
                        {
                            "type": 145,
                            "typeName": "<T::Lookup as StaticLookup>::Source"
                        },
                        {
                            "type": 68,
                            "typeName": "T::Balance"
                        }
                    ]
                },
                {
                    "name": "transfer_all",
                    "fields": [
                        {
                            "type": 145,
                            "typeName": "<T::Lookup as StaticLookup>::Source"
                        },
                        {
                            "type": 34,
                            "typeName": "bool"
                        }
                    ]
                }
            ]
        }
    },
    "docs": [
        "r\"Contains one variant per dispatchable that can be called by an extrinsic."
    ]
}
```

This shows us the entire inner enum, and we can look at the types of the two params for `transfer` (#145 and #168)
to dig further into it (`jq '.[1].V14.types.types[145]'` and `jq '.[1].V14.types.types[168]'`). If we do this, we'll
see that our extrinsic should be comprised of the following properties (in order):

- Outer call variant index; which pallet are we calling into (`5: u8`) (index of "Balances" pallet in result from 03_metadata)
- Inner call enum variant index (`0; u8`) (index of "transfer" call in result from 03_metadata)
- A "MultiAddress", which we can dig into and see that it consists of:
  - A variant index to say we want to provide an `AccoundId32` (`0: u8`)
  - The actual address (`[u8; 32]`)
  (But we'll just import and use the MultiAddress type, which encodes to the same as above)
- A Balance, which is a `u128`

So, a type like `(u8, u8, u8, [u8; 32], u128)` will encode to the correct bytes to represent the call we want to make.
If a call doesn't need to be signed, we can just prepend a `None` signature to it (`0; u8`). If it does need to be
signed, we'll need to gather and sign some details, including our call data, and prepend this signature/validity information.

When this is all constructed, we can SCALE encode, generate a hex string, and submit that to the node API. The code
below runs through this.
*/

use std::str::FromStr;

use utils::rpc_to_localhost;
use sp_core::{H256, blake2_256};
use sp_keyring::AccountKeyring;
use sp_runtime::{MultiAddress, MultiSignature, generic::Era};
use sp_version::RuntimeVersion;
use parity_scale_codec::{ Compact, Encode };

#[tokio::main]
async fn main() {

    // So, given the above, our call is composed of these values:
    let pallet_index: u8 = 5;
    let call_index: u8 = 0;
    let address = MultiAddress::Id::<_,u32>(AccountKeyring::Bob.to_account_id());
    let balance = Compact::from(123456789012345u128);

    // Values are provided in the following order (pallet and call index, and then args).
    // This encodes to the same format as using the Call variants directly would.
    let call = (
        pallet_index,
        call_index,
        address,
        balance,
    );

    // Extra information related to the call. We'll need to include this information
    // when we submit the call.
    let extra = (
        // How long should this call "last" in the transaction pool before
        // being deemed "out of date" and discarded?
        Era::Immortal,
        // How many prior transactions have occurred from this account? This
        // Helps protect against replay attacks or accidental double-submissions.
        Compact::<u32>(0),
        // I'm not sure what this is at the moment, but I've seen it just set to 0.
        Compact::<u128>(0)
    );

    // A little more info we need for the additional info:
    let runtime_version = get_runtime_version().await;
    let genesis_hash = get_genesis_hash().await;

    // We want to sign the payload against this additional information,
    // but we won't be including it in the final signed payload:
    let additional = (
        runtime_version.spec_version,
        runtime_version.transaction_version,
        genesis_hash,
        genesis_hash,
        (),
        (),
        ()
    );

    // Sign the data with Alice's private key
    let signature = {
        // Combine this data together and SCALE encode it:
        let full_unsigned_payload = (&call, &extra, &additional);
        let full_unsigned_payload_scale_bytes = full_unsigned_payload.encode();

        // If payload logner than 256 bytes, we hash it and sign the hash instead:
        if full_unsigned_payload_scale_bytes.len() > 256 {
            AccountKeyring::Alice.sign(&blake2_256(&full_unsigned_payload_scale_bytes)[..])
        } else {
            AccountKeyring::Alice.sign(&full_unsigned_payload_scale_bytes)
        }
    };

    // This is the format of the signature that we'll want to encode:
    let signature_to_encode = Some((
        MultiAddress::Id::<_,u32>(AccountKeyring::Alice.to_account_id()),
        MultiSignature::Sr25519(signature),
        extra
    ));

    // Encode it using logic borrowed from substrate-api-sidecar:
    let payload_scale_encoded = encode_extrinsic(
        signature_to_encode,
        call
    );
    let payload_hex = format!("0x{}", hex::encode(&payload_scale_encoded));

    println!("Submitting this payload: {}", payload_hex);

    // Submit it!
    let res = rpc_to_localhost("author_submitExtrinsic", [payload_hex]).await.unwrap();

    println!("{:?}", res);
}

async fn get_genesis_hash() -> H256 {
    let genesis_hash_json = rpc_to_localhost("chain_getBlockHash", [0]).await.unwrap();
    let genesis_hash_hex = genesis_hash_json.as_str().unwrap();
    H256::from_str(genesis_hash_hex).unwrap()
}

async fn get_runtime_version() -> RuntimeVersion {
    let runtime_version_json = rpc_to_localhost("state_getRuntimeVersion", ()).await.unwrap();
    serde_json::from_value(runtime_version_json).unwrap()
}

/// Adapted from substrate-api-client; we'll mirror how they encode extrinsics:
fn encode_extrinsic<S: Encode ,C: Encode>(signature: Option<S>, call: C) -> Vec<u8> {
    encode_with_vec_prefix::<(S,C), _>(|v| {
        const V4: u8 = 4;
        match signature.as_ref() {
            Some(s) => {
                v.push(V4 | 0b1000_0000);
                s.encode_to(v);
            }
            None => {
                v.push(V4 & 0b0111_1111);
            }
        }
        call.encode_to(v);
    })
}

/// Copied from substrate-api-client; we'll mirror how they encode extrinsics.
fn encode_with_vec_prefix<T: Encode, F: Fn(&mut Vec<u8>)>(encoder: F) -> Vec<u8> {
    let size = std::mem::size_of::<T>();
    let reserve = match size {
        0..=0b0011_1111 => 1,
        0b0100_0000..=0b0011_1111_1111_1111 => 2,
        _ => 4,
    };
    let mut v = Vec::with_capacity(reserve + size);
    v.resize(reserve, 0);
    encoder(&mut v);

    // need to prefix with the total length to ensure it's binary compatible with
    // Vec<u8>.
    let mut length: Vec<()> = Vec::new();
    length.resize(v.len() - reserve, ());
    length.using_encoded(|s| {
        v.splice(0..reserve, s.iter().cloned());
    });

    v
}