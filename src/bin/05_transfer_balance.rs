/*!

Run with:

```
cargo run --bin 05_transfer_balance
```

An extrinsic is something that ends up in a block. The data in an extrinsic describes a state change, such
that a new node can download and replay all the blocks and end up in the same state. Extrinsics either
come from the outside world ("transactions") or come from within the node itself ("inherents").

Transactions can be submitted via the RPC call "author_submitExtrinsic". They are hex encoded SCALE encoded
bytes which take something like the following format:

- Compact encoded number of SCALE encoded bytes following this.
- 1 bit: a 0 if no signature is present, or a 1 if it is.
- 7 bits: the transaction protocol version; these docs are all about version 4, and so expect that.
- If there is a signature:
  - a SCALE encoded `sp_runtime::MultiAddress::Id<AccountId32, u32>`; who is the transaction from.
  - a SCALE encoded `sp_runtime::MultiSignature::S225519`; a signature (see the code for how this is made)
  - a SCALE encoded `sp_runtime::generic::Era`; how long will this transaction live in the pool for.
  - Compact encoded u32: how many transactions have occurred from the "from" address already.
  - Compact encoded u128: a tip paid to the block producer.
- The call data, which consists of:
  - 1 byte: the pallet index we're calling into.
  - 1 byte: the function in the pallet that we're calling.
  - variable: the SCALE encoded parameters required by the function being called.

The call data is the "meat" of the transaction and describes the actual thing we want done (eg transferring DOT).
Look in the `substrate/frame` for the pallet code; exposed calls are decorated, and we can see the parameters
they take. (Or, look in the metadata to see the same).

For example, the call to transfer money from one account to another, is in in the Balances pallet
(`substrate/frame/balances/src/lib.rs`), and looks like:

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

When constructing our call data to submit, our goal is to select the call we'd like to make from these, and
create a SCALE encoded representation of that enum variant ourselves.

# Metadata

The metadata contains all of the information we need to know how to construct these transactions ourselves (and more).
`jq` is useful for looking through it. Examples:

```sh
# Get the name and index of all pallets in the runtime:
cargo run --bin 03_metadata | jq '.[1].V14.pallets[] | { name: .name, index: .index }'
# get details of the pallet at some index (5, here):
cargo run --bin 03_metadata | jq '.[1].V14.pallets[] | select(.index == 5)'
# Get information about the extrinsic type (signed_extensions):
cargo run --bin 03_metadata | jq '.[1].V14.extrinsic'
# Get information abotu a specific type, eg type ID 676:
cargo run --bin 03_metadata | jq '.[1].V14.types.types[676]'
```

Read the source below for more on this.
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

    // The transaction is coming from Alice.
    let from = AccountKeyring::Alice.to_account_id();

    // The "transfer" call takes 2 arguments, which are as follows (if we wanted, we could
    // avoid using `MultiAddress` and encode a 0 u8 and then the account ID, but for simplicity..)
    let address = MultiAddress::Id::<_,u32>(AccountKeyring::Bob.to_account_id());
    let balance = Compact::from(123456789012345u128);

    // We're transferring the money from Alice. How many transfers has she made already? we need
    // to include this number below; it has to be correct for the transfer to succeed.
    let alice_nonce = get_nonce(&from).await;

    // We put the above data together and now we have something that will encode to the
    // Same shape as the generated enum would have led to (variant indexes, then args):
    let call = (
        pallet_index,
        call_index,
        address,
        balance,
    );

    // As well as the call data above, we need to include some extra information along
    // with our transaction. See the "signed_extension" types here to know what we need to
    // include:
    //
    // cargo run --bin 03_metadata | jq '.[1].V14.extrinsic'
    //
    // Many "ty" props there will resolve to nothing, so can be ignored. The ones that don't
    // resolve to nothing are the ones that encode to a non-zero number of bytes and need to
    // therefore be included.
    let extra = (
        // How long should this call "last" in the transaction pool before
        // being deemed "out of date" and discarded?
        Era::Immortal,
        // How many prior transactions have occurred from this account? This
        // Helps protect against replay attacks or accidental double-submissions.
        Compact(alice_nonce),
        // This is a tip, paid to the block producer (and in part the treasury)
        // to help incentive it to include this transaction in the block. Can be 0.
        Compact(500000000000000u128)
    );

    // Grab a little more info that we'll need for below:
    let runtime_version = get_runtime_version().await;
    let genesis_hash = get_genesis_hash().await;

    // This information won't be included in our payload, but is it part of the data
    // that we'll sign, to help ensure that the TX is only valid in the right place.
    // See the "signed_extension" types here to know what we need to include:
    //
    // cargo run --bin 03_metadata | jq '.[1].V14.extrinsic'
    //
    // Look at the "additional_signed" type IDs now. Any that resolve to a type that
    // encodes to a non-zero number of bytes needs to be included.
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
        MultiAddress::Id::<_,u32>(from),
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

/// How many transactions has this account already made?
async fn get_nonce(account: &sp_runtime::AccountId32) -> u32 {
    let nonce_json = rpc_to_localhost("system_accountNextIndex", (account,)).await.unwrap();
    serde_json::from_value(nonce_json).unwrap()
}

/// Encode the extrinsic into the expected format. De-optimised a little
/// for simplicity, and taken from sp_runtime/src/generic/unchecked_extrinsic.rs
fn encode_extrinsic<S: Encode ,C: Encode>(signature: Option<S>, call: C) -> Vec<u8> {
    let mut tmp: Vec<u8> = vec![];

    // 1 byte for version ID + "is there a signature".
    // The top bit is 1 if signature present, 0 if not.
    // The remaining 7 bits encode the version number (here, 4).
    const EXTRINSIC_VERSION: u8 = 4;
    match signature.as_ref() {
        Some(s) => {
            tmp.push(EXTRINSIC_VERSION | 0b1000_0000);
            // Encode the signature itself now if it's present:
            s.encode_to(&mut tmp);
        },
        None => {
            tmp.push(EXTRINSIC_VERSION & 0b0111_1111);
        },
    }

    // Encode the call itself after this version+signature stuff.
    call.encode_to(&mut tmp);

    // We'll prefix the encoded data with it's length (compact encoding):
    let compact_len = Compact(tmp.len() as u32);

    // So, the output will consist of the compact encoded length,
    // and then the version+"is there a signature" byte,
    // and then the signature (if any),
    // and then encoded call data.
    let mut output: Vec<u8> = vec![];
    compact_len.encode_to(&mut output);
    output.extend(tmp);

    output
}
