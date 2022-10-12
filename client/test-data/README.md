Contains info about the individual communities.

# Polkadot LATAM DEMO
* Bootstrappers are the seeds with:
   * //LATAM1
   * //LATAM2
   * //LATAM3


## Demo Flow 1:1
Demo is based on the encointer book: https://book.encointer.org/tutorials-register-community.html

### Preliminaries
1. Fill out bootstrappers in the community spec file
2. Pick locations with https://geojson.io and insert them into the spec file
3. Upload assets to IPFS and insert the CID into the spec file
4. Prepare the App with //LATAM1 account
### Demo
```bash
# in the client folder of this repository
cd encointer-node/client

NURL=wss://gesell.encointer.org
NPORT=443
ENCOINTER_CLIENT_BINARY=../target/release/encointer-client-notee
alias nctr-gsl="$ENCOINTER_CLIENT_BINARY -u $NURL -p $NPORT"

# faucet //LATAM1, //LATAM2, LATAM3
nctr-gsl faucet 5H1CeCqNSpJPRLScQb9jz5ES7j6vL8sP8Ai7J7f3sJHWkTek 5GjJjBPg8XzD2RMzFSV2Qq42CxBdJsND9fRoBtxCqmYNJA4M 5D83c6U4cpnJRUFi9hZZroBPzB2g2sd91eFT3Rm2QTp7ZJau

nctr-gsl new-community ./test-data/latam.hackathon.json --signer //LATAM1

# register //LATAM1, //LATAM2, LATAM3
# (register the //LATAM1 IN THE APP)
nctr-gsl register-participant //LATAM1 --cid 3zz704jWojt
nctr-gsl register-participant //LATAM2 --cid 3zz704jWojt
nctr-gsl register-participant //LATAM3 --cid 3zz704jWojt
 
# go to assigning phase with sudo key and show the meetup location in app
nctr-gsl next-phase 5CSLXnYZQeVDvNmanYEJn4YXXhgFLKYwp2f216NsDehR8mVU

# go to attesting phase
nctr-gsl next-phase 5CSLXnYZQeVDvNmanYEJn4YXXhgFLKYwp2f216NsDehR8mVU

# perform latam meetup with rust cli-wrapper.
./cli.py perform-latam-meetup-gsl


# go to registering phase and claim meetup with the app
nctr-gsl next-phase 5CSLXnYZQeVDvNmanYEJn4YXXhgFLKYwp2f216NsDehR8mVU

```

