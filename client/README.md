# encointer CLI client
Interact with the encointer chain from the command line

Includes
* keystore (incompatible with polkadot js app json)
* basic balance transfer
* all encointer-specific calls

## examples
```
> encointer-client new_account
> encointer-client 127.0.0.1 transfer 5GpuFm6t1AU9xpTAnQnHXakTGA9rSHz8xNkEvx7RVQz2BVpd 5FkGDttiYa9ZoDAuNxzwEdLzkgt6ngWykSBhobGvoFUcUo8B 12345
> encointer-client 127.0.0.1:9979 register_participant 5FkGDttiYa9ZoDAuNxzwEdLzkgt6ngWykSBhobGvoFUcUo8B
> encointer-client 127.0.0.1:9979 list_participant_registry
> encointer-client 127.0.0.1:9979 get_phase
> encointer-client 127.0.0.1:9979 new_claim 5EqvwjCA8mH6x9gWbSmcQhxDkYHJcUfwjaHHn9q1hBrKLL65 3
> encointer-client 127.0.0.1:9979 sign_claim 5EqvwjCA8mH6x9gWbSmcQhxDkYHJcUfwjaHHn9q1hBrKLL65 7af690ced4cd1e84a857d047b4fc93f3b4801f9a94c9a4d568a01bc435f5bae903000000000000000000000003000000
```

Find a full ceremony cycle demo [here](./bootstrap_demo_community.py)

# run a local bot community benchmark

start encointer blockchain in dev mode
```bash
./target/release/encointer-node-notee --tmp --dev --enable-offchain-indexing true -lencointer=debug
```

start faucet service
```bash
cd client
./faucet.py
```

initialize bot community
```bash
cd client
./bot-community.py init
```

start phase controller service (fast forwards phase after N idle blocks)
```bash
cd client
./phase.py
```

listen to chain events for debugging (i.e. see failed extrinsics)
```bash
RUST_LOG=encointer_client_notee=info ./target/release/encointer-client-notee listen
```

run a single cycle
```bash
cd client
./bot-community.py run
```

benchmark bot community
```bash
cd client
./bot-community.py benchmark
```

if you'd like to test bazaar with dummy businesses and offerings too, you need to provide IPFS.

either through infura

```
export IPFS_ADD_URL=https://ipfs.infura.io:5001/api/v0/add
export IPFS_API_KEY=<user>:<password>
./bot-communities.py init
./register-businesses.py
```

or locally

```
# you may need to run 'ipfs init'
ipfs daemon
./bot-communities.py init --ipfs-local
./register-businesses.py  --ipfs-local
```

In IPFS, the community icons and data of businesses and offerings are stored.
## Notes

The cli provides helpful information for the ordering of options and commands. <br>
You can use a custom chain client by providing the option before the command
```
./bot-communities.py --client ./path/to/custom/client init
```
You can also connect to a remote chain
```
./bot-communities.py init --node_url wss://remote.node.org
```
For now, the node_url is hardcoded to 'wss://gesell.encointer.org' despite which value you enter. This will be changed in the future when more remote chains will be available. 
