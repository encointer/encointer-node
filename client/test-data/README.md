Contains info about the individual communities.

# LATAM
* Bootstrappers are the seeds with:
   * //LATAM1
   * //LATAM2
   * //LATAM3


## Demo Flow 1:1
### Preliminaries
1. Pick locations with https://geojson.io and insert them into the spec file
2. Upload assets to IPFS
3. Prepare the App with //LATAM1 account
### Demo
```bash
# launch de encointer-node
./target/release/encointer-node-notee --dev --enable-offchain-indexing true --rpc-methods unsafe -lencointer=debug,parity_ws=warn --ws-external --rpc-external

# from now on in other terminal window
cd encointer-node/client

alias nctr-dev="../target/release/encointer-client-notee"`

# faucet //LATAM1, //LATAM2, LATAM3
nctr-dev faucet 5H1CeCqNSpJPRLScQb9jz5ES7j6vL8sP8Ai7J7f3sJHWkTek 5GjJjBPg8XzD2RMzFSV2Qq42CxBdJsND9fRoBtxCqmYNJA4M 5D83c6U4cpnJRUFi9hZZroBPzB2g2sd91eFT3Rm2QTp7ZJau

nctr-dev new-community ./test-data/latam.hackathon.json --signer //LATAM1

# register //LATAM1, //LATAM2, LATAM3
# (register the //LATAM1 IN THE APP)
nctr-dev register-participant //LATAM1 --cid 3zz704jWojt
nctr-dev register-participant //LATAM2 --cid 3zz704jWojt
nctr-dev register-participant //LATAM3 --cid 3zz704jWojt
 
# go to assigning phase and show the meetup location in app
nctr-dev next-phase

# go to attesting phase
nctr-dev next-phase

# perform latam meetup with rust cli-wrapper.
./cli.py perform-latam-


# go to registering phase and claim meetup with the app
nctr-dev next-phase
```

