A few businesses to serve as examples

# how to register a single business

## register a pure proxy for the business

not mandatory, but good practice

[PJS call](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fencointer-kusama-rpc.n.dwellir.com#/extrinsics/decode/0x2c040000000000e803)
make sure to adjust the index to your needs. You can choose freely, but uniquely.
check events to learn the AccountId for your new proxy

## upload assets to IPFS

On your IPFS node:

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

## register business on chain

`proxy.proxy(encointerBazaar.createBusiness(community_cid, url_or_ipfs_cid)`

[PJS call](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fencointer-kusama-rpc.n.dwellir.com#/extrinsics/decode/0x2c000001b6a163db2ac1bdd8da5a3870d9149e9fdad0e643cf78f1eabe70bf149a99190040007530716a3977f79df7b8516d62336d5259524b366e7766334d58554c50524841514841666b47733338554a37766f584c504e39676e677161)

you should see this event: `encointerBazaar.BusinessCreated`
