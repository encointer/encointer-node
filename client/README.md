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

execute the current phase (without advancing to the next phase)
```bash
cd client
./bot-community.py execute-current-phase
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
./bot-communities.py --ipfs-local init 
./register-businesses.py --ipfs-local
```

In IPFS, the community assets and data of businesses and offerings are stored.

You can cat/get the data stored in ipfs locally:
```
ipfs cat <CONTENT_IDENTIFIER>
```
Or if it was stored remotely (on Infura):
```
curl -X POST "https://ipfs.infura.io:5001/api/v0/cat?arg=<CONTENT_IDENTIFIER>" 
```

# run bot-community against parachain

The same client can be used against an encointer parachain. Please start the parachain with polkadot-launch from the encointer-parachain repo:
```
node ../polkadot-launch/dist/cli.js polkadot-launch/launch-kusama-local-with-encointer.json
```

then you can run bootstrapping or bot-community exactly like for solo-node

# register real community

## upload assets to IPFS

Make sure your SVG community icon doesn't include `<style>` attributes

```
cargo install svgcleaner
svgcleaner community_icon.svg community_icon.svg
```

using infura:
```
ipfs-upload-client --id <your infura id> --secret <your infura secret> --pin leu.rococo 
```

using your own ipfs server
```
ipfs add -rw --pin leu.rococo
```
test if you can fetch the cid through the encointer gateway which will be used by the app

```
wget http://ipfs.encointer.org:8080/api/v0/object/get?arg=QmXydp7gdTGwxkCn24vEtvtSXbR7wSAGBDLQpc8buF6T92/community_icon.svg
```
it may take a while to sync from the server you used for uploading and pinning

## create you community spec file

insert your asset ipfs cid from above

create a proposal:
```
RUST_LOG=info ../target/release/encointer-client-notee -u wss://rococo.api.encointer.org -p 443 new-community test-data/leu.rococo.json
```
