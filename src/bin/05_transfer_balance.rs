/*!

Run with:

```
cargo run --bin 05_transfer_balance
```

An extrinsic consists of call information (what function are we calling from which pallet, and which
arguments does that function want to be given) and optionally (but likely) also signature information
(who is submitting the extrinsic, the signature itself, and extra information about where/how long is
it valid for). All of this information is SCALE encoded in a particular format, converted to a hex string,
and submitted via JSONRPC to action the extrinsic.

Extrinsics are generated from code in the `substrate/frame` folder. For example, to transfer money from
one account to another, we need to construst an extrinsic which ends up calling the `transfer` method
in the Balances pallet (`substrate/frame/balances/src/lib.rs`), whose signature looks like:

```no_run
pub fn transfer(
    origin: OriginFor<T>,
    dest: <T::Lookup as StaticLookup>::Source,
    #[pallet::compact] value: T::Balance,
)
```

There is a macro applied on these functions which will generate a `Call` enum that contains, amoung other
variants, a `transfer` variant, whose arguments are identical to those seen in the function definition
(ignoring origin, which is separate and known by way of the extrinsic signature). We can see this by going
to `substrate/frame/balances` and running `cargo doc --open`. It looks something like:

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

However, there are more pallets than just Balances, so not only do we have this inner enum, which describes
the method in the pallet you'd like to call and the data to provide to it, but we have an outer enum which
contains this, and others, to describe all of the calls from every pallet. For a Polkadot node, we can see this
enum by running `cargo doc --open` in `polkadot/runtime/polkadot` and finding the outer `Call` enum there.

So, if we were to construct one of these enum variants we'd have part of the information we want to submit
(the signature details being the other part). However, we don't actually need to include all of `polkadot`
in order to have access to this generated enum; we just need to encode data that will have the same encoded
shape as the enum.

With that all said, a running node can hand us metadata (see the example 03_metadata) which, as of V14 (which
is soon to be released) will actually contain all of the type information needed to construct a valid extrinsic
without us needing to dig through substrate/polkadot types to figure out what the concrete type of things like
<T::Lookup as StaticLookup>::Source` or `T::Balance` are (which we'd need to do if constructing a balance
transfer call).

So, how can we actually construct a valid extrinsic from the upcoming V14 metadata then?

Step 1: Construct our call data in a format that encodes to the same as the Call enum variant we'd want.
Using some example typed metadata from `https://gist.github.com/ascjones/b76a5345930776ede61dd0f797792ed4`,
let's see what we can find out about a call to transfer some balance.

First, the "transfer" call in the "Balances" module looks a bit like this in the V14 metadata:

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

This is describing the entire inner enum that's generated for the Balances pallet. We can look at the types
of the two params for `transfer` (#145 and #168) to dig further into it (`jq '.[1].V14.types.types[145]'`
and `jq '.[1].V14.types.types[168]'`). If we do this, we can surmize that our extrinsic should be comprised of
the following properties (in order):

- Outer call variant index; which pallet are we calling into. If we find the index of "Balances" pallet in the
  result from 03_metadata, we can see that it's index 5, and that'll be a `u8` to mirror the variant tag).
- Inner call enum variant index. We can see that the `transfer` call is the first variant from the metadata, so
  that'll be index 0, also a `u8`.
- A "MultiAddress", which we can dig into and see that it consists of:
  - A variant index (which will be 0) to say we want to provide an `AccoundId32`
  - The actual address (with the type `[u8; 32]`)
  (But we'll just import and use the MultiAddress type for simplicity in our example, rather than manually
  encoding something of the same shape)
- A Balance, which is a `u128` (but compact encoded; see SCALE encoding docs for details on what this means).

So, a type like `(u8, u8, u8, [u8; 32], u128)` will encode to the correct bytes to represent the call we want
to make. If a call doesn't need to be signed, we can just prepend a `None` signature to it (`0; u8`). If it
does need to be signed, we'll need to gather and sign some details, including our call data, and prepend this
signature/validity information. We can also attach extra information alongside the signature.

When that's obtained, we encode the data in a specific way, and then we can send it off to be executed. The
following example constructs and sends off a "balances.transfer" extrinsic:
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

    // First, we need to know which pallet, and which call in the pallet, we're
    // actually calling. This equates to seeing which index in the arrays in the
    // metadata the "Balances" pallet and then the "transfer" call are at, but for
    // simplicity I've just manually had a look and hard coded them here:
    let pallet_index: u8 = 5;
    let call_index: u8 = 0;

    // The "transfer" call takes 2 arguments, which are as follows (if we wanted, we could
    // avoid using `MultiAddress` and encode a 0 u8 and then the account ID, but for simplicity..)
    let address = MultiAddress::Id::<_,u32>(AccountKeyring::Bob.to_account_id());
    let balance = Compact::from(123456789012345u128);

    // We put the above data together and now we have something that will encode to the
    // Same shape as the generated enum would have led to (variant indexes, then args):
    let call = (
        pallet_index,
        call_index,
        address,
        balance,
    );

    // As well as the call data above, we need to include some extra information along
    // with our transaction:
    let extra = (
        // How long should this call "last" in the transaction pool before
        // being deemed "out of date" and discarded?
        Era::Immortal,
        // How many prior transactions have occurred from this account? This
        // Helps protect against replay attacks or accidental double-submissions.
        Compact(0u32),
        // This is a tip, paid to the block producer (and in part the treasury)
        // to help incentive it to include this transaction in the block. Can be 0.
        Compact(500000000000000u128)
    );

    // Grab a little more info that we'll need for below:
    let runtime_version = get_runtime_version().await;
    let genesis_hash = get_genesis_hash().await;

    // This information won't be included in our payload, but is it part of the data
    // that we'll sign, to help ensure that the TX is only valid in the right place.
    let additional = (
        // This TX won't be valid if it's not executed on the expected runtime version:
        runtime_version.spec_version,
        runtime_version.transaction_version,
        // Genesis hash, so TX is only valid on the correct chain:
        genesis_hash,
        // The block hash of the "checkpoint" block. If the transaction is
        // "immortal", use the genesis hash here. If it's mortal, this block hash
        // should be equal to the block number provided in the Era information,
        // so that the signature can verify that we're looking at the expected block.
        // (one thing that this can help prevent is your transaction executing on the
        // wrong fork; same genesis hash but likely different block hash for mortal tx).
        genesis_hash,
    );

    // Now, we put the data we've gathered above together and sign it:
    let signature = {
        // Combine this data together and SCALE encode it:
        let full_unsigned_payload = (&call, &extra, &additional);
        let full_unsigned_payload_scale_bytes = full_unsigned_payload.encode();

        // If payload is longer than 256 bytes, we hash it and sign the hash instead:
        if full_unsigned_payload_scale_bytes.len() > 256 {
            AccountKeyring::Alice.sign(&blake2_256(&full_unsigned_payload_scale_bytes)[..])
        } else {
            AccountKeyring::Alice.sign(&full_unsigned_payload_scale_bytes)
        }
    };

    // This is the format of the signature part of the transaction. If we want to
    // experiment with an unsigned transaction here, we can set this to None::<()> instead.
    let signature_to_encode = Some((
        // The account ID that's signing the payload:
        MultiAddress::Id::<_,u32>(AccountKeyring::Alice.to_account_id()),
        // The actual signature, computed above:
        MultiSignature::Sr25519(signature),
        // Extra information to be included in the transaction:
        extra
    ));

    // Encode the extrinsic, which amounts to combining the signature and call information
    // in a certain way:
    let payload_scale_encoded = encode_extrinsic(
        signature_to_encode,
        call
    );
    let payload_hex = format!("0x{}", hex::encode(&payload_scale_encoded));

    // Submit it!
    println!("Submitting this payload: {}", payload_hex);
    let res = rpc_to_localhost("author_submitExtrinsic", [payload_hex]).await.unwrap();

    // The result from this call is the hex value for the extrinsic hash.
    println!("{:?}", res);
}

/// Fetch the genesis hash from the node.
async fn get_genesis_hash() -> H256 {
    let genesis_hash_json = rpc_to_localhost("chain_getBlockHash", [0]).await.unwrap();
    let genesis_hash_hex = genesis_hash_json.as_str().unwrap();
    H256::from_str(genesis_hash_hex).unwrap()
}

/// Fetch runtime information from the node.
async fn get_runtime_version() -> RuntimeVersion {
    let runtime_version_json = rpc_to_localhost("state_getRuntimeVersion", ()).await.unwrap();
    serde_json::from_value(runtime_version_json).unwrap()
}

/// Encode the extrinsic into the expected format. De-optimised a little
/// for simplicity, and taken from sp_runtime/src/generic/unchecked_extrinsic.rs
fn encode_extrinsic<S: Encode ,C: Encode>(signature: Option<S>, call: C) -> Vec<u8> {
    let mut tmp: Vec<u8> = vec![];

    // 1 byte version id; a combination of extrinsic version and
    // whether or not there's a signature in the response.
    const EXTRINSIC_VERSION: u8 = 4;
    match signature.as_ref() {
        Some(s) => {
            tmp.push(EXTRINSIC_VERSION | 0b1000_0000);
            s.encode_to(&mut tmp);
        },
        None => {
            tmp.push(EXTRINSIC_VERSION & 0b0111_1111);
        },
    }
    call.encode_to(&mut tmp);

    // We'll prefix the encoded data with it's length (compact encoding):
    let compact_len = Compact(tmp.len() as u32);

    // So, the output will consist of the compact encoded length,
    // and then the 1 byte version+"is there a signature" byte,
    // and then the signature (if any) and then encoded call data.
    let mut output: Vec<u8> = vec![];
    compact_len.encode_to(&mut output);
    output.extend(tmp);

    output
}
