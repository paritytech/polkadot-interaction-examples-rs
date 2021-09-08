# Examples of interacting with a Polkadot node

Some examples of using JSON RPC to interact with a Polkadot node, working up to manually building and submitting a balance transfer.

To run these examples, first start up a local Polkadot node (which we'll be interacting with):

```
# Clone the polkadot repo:
git clone https://github.com/paritytech/polkadot.git
# This is the commit I used (the examples will likely break as master is updated):
git checkout f3f83e3f9db049f981066b3a94fa17cad673299f
# Start up a polkadot dev node on your machine:
cargo run -- --tmp --dev
```

Once you have this node running, in another terminal, pick an example you'd like to run from the `src/bin` folder and run it like so:

```
cargo run --bin 01_basic
```

Note that the balance transfer example expects a fresh dev node (the transaction has a nonce which means it can't be executed more than once). Just restart the Polkadot node to get back to a fresh state.

The examples are well commented, so check them out to find out more!

One really useful tip for debugging what is happening is to visit https://polkadot.js.org/apps, and point it at your local node (click top left corner and in the "development" tab, point it to the node at `ws://127.0.0.1:9944`). Using this, you can see the balance transfer in example 05 take place, and compare other results with the actual node state.