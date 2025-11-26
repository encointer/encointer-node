A few businesses to serve as examples

# how to register a single business

## register a pure proxy for the business

not mandatory, but good practice

[PJS call](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fencointer-kusama-rpc.n.dwellir.com#/extrinsics/decode/0x2c040000000000e803)
make sure to adjust the index to your needs. You can choose freely, but uniquely.
check events to learn the AccountId for your new proxy

## upload assets to IPFS

Run a local ipfs daemon:

```
> ipfs daemon
```
we will pin the content on a remote node later

```
> ipfs add -r assets --pin                                                                             
added QmbAsammnMX41xiJPVVhLTQB6UaMPyYPFgpZVg8qBTGWNE assets/logo.png
added QmVEA2LhinFNjkqFgt7USPiMEpSt1SrJQDf5BsDYi52jaF assets/photos/image01.png
added Qme5WiVzgjPbgmzPobB41FXQM3Y2Pen47YjuQ9hiZxTfDt assets/photos/image02.jpg
added QmXvia78N8xrLEEjipiEjibRJ1JTpUEqop6XCQZNcE3PTC assets/photos/image03.jpg
added QmasSnnY6w6tMYYFzC5xaHa9GrhmeZ99aGx3eXD2rqpz8b assets/photos
added QmUxPhjtx7NxByaD6UwFzz46oeubShmL9mNMqAuM72mQTq assets
```

note the assets/photos cid and assets/logo.png in the business json file:

for reference: [the data model we use](https://github.com/encointer/pallets/tree/master/bazaar#scope): [LocalBusiness](https://schema.org/LocalBusiness)
supported categories ATM: `art_music, body_soul, fashion_clothing, food_beverage_store, restaurants_bars, it_hardware, food, other`

```json
{
  "name": "Revamp-IT",
  "description": "Computersupport und -dienste",
  "category": "it_hardware",
  "address": "Birmensdorferstrasse 379, 8055 Zürich",
  "telephone": null,
  "email": null,
  "longitude": "8.5049619",
  "latitude": "47.3690377",
  "openingHours": "Mon 9h-12h, Tue-Fri 13h-17h",
  "logo": "QmbAsammnMX41xiJPVVhLTQB6UaMPyYPFgpZVg8qBTGWNE",
  "photos": "QmasSnnY6w6tMYYFzC5xaHa9GrhmeZ99aGx3eXD2rqpz8b"
}
```

then, pin that cid on your ipfs node:

```
> ipfs add revamp-it.json --pin
added Qmb3mRYRK6nwf3MXULPRHAQHAfkGs38UJ7voXLPN9gngqa revamp-it.json
``` 
## make available via a public ipfs gateway

connect your local node to Encointer's public gateway:

```
ipfs swarm connect /ip4/129.212.213.82/tcp/4001/p2p/12D3KooWCtS4Li5YJhjj2fWWKxgqZi7t6FReuVtrN9MRQtLrg7Tj
# pin assets/photos recursively: 
curl -u <USER>:<PWD> -s -X POST "https://ipfs2-api.encointer.org/api/v0/pin/add?recursive=true&arg=QmasSnnY6w6tMYYFzC5xaHa9GrhmeZ99aGx3eXD2rqpz8b"
# pin biz metadata
curl -u <USER>:<PWD> -s -X POST "https://ipfs2-api.encointer.org/api/v0/pin/add?recursive=true&arg=Qmb3mRYRK6nwf3MXULPRHAQHAfkGs38UJ7voXLPN9gngqa"
```

or pin on any other ipfs node you have access to:

```
ipfs pin add -r QmasSnnY6w6tMYYFzC5xaHa9GrhmeZ99aGx3eXD2rqpz8b
ipfs pin add -r Qmb3mRYRK6nwf3MXULPRHAQHAfkGs38UJ7voXLPN9gngqa
```
  
## register business on chain

Use the metadata cid to register your biz: `Qmb3mRYRK6nwf3MXULPRHAQHAfkGs38UJ7voXLPN9gngqa` using
`proxy.proxy(encointerBazaar.createBusiness(community_cid, url_or_ipfs_cid)`

[PJS call](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fencointer-kusama-rpc.n.dwellir.com#/extrinsics/decode/0x2c000001b6a163db2ac1bdd8da5a3870d9149e9fdad0e643cf78f1eabe70bf149a99190040007530716a3977f79df7b8516d62336d5259524b366e7766334d58554c50524841514841666b47733338554a37766f584c504e39676e677161)

you should see this event: `encointerBazaar.BusinessCreated`
