#!/usr/bin/env python3
"""
Demonstrate the bootstrapping of an Encointer community on a *dev* chain.

start node with
  ../target/release/encointer-node-notee --dev --tmp --ws-port 9945 --enable-offchain-indexing true --rpc-methods unsafe

then run this script
  ./bootstrap_demo_community.py --port 9945

"""

import argparse
import json
import os

from py_client.arg_parser import simple_parser
from py_client.client import Client
from py_client.scheduler import CeremonyPhase
from py_client.ipfs import Ipfs, ICONS_PATH
from py_client.helpers import zip_folder

account1 = '//Alice'
account2 = '//Bob'
account3 = '//Charlie'
accounts = [account1, account2, account3]

TEST_DATA_DIR = '../test-data/'
SPEC_FILE = 'test-locations-mediterranean.json'


def perform_meetup(client, cid):
    print('Starting meetup...')
    print('Creating claims...')
    vote = len(accounts)
    claim1 = client.new_claim(account1, vote, cid)
    claim2 = client.new_claim(account2, vote, cid)
    claim3 = client.new_claim(account3, vote, cid)

    print('Sending claims of attestees to chain...')
    client.attest_claims(account1, [claim2, claim3])
    client.attest_claims(account2, [claim1, claim3])
    client.attest_claims(account3, [claim1, claim2])


def update_spec_with_cid(file, cid):
    with open(file, 'r+') as spec_json:
        spec = json.load(spec_json)
        spec['community']['meta']['icons'] = cid
        print(spec)
        # go to beginning of the file to overwrite
        spec_json.seek(0)
        json.dump(spec, spec_json, indent=2)
        spec_json.truncate()


def main(ipfs_local, client=Client()):
    spec_file_path = f'{TEST_DATA_DIR}{SPEC_FILE}'

    cid = client.new_community(spec_file_path, account1)
    print(f'Registered community with cid: {cid}')

    print('Uploading icons to ipfs')
    root_dir = os.path.realpath(ICONS_PATH)
    zipped_folder = zip_folder("icons",root_dir)
    ipfs_cid = Ipfs.add(zipped_folder, ipfs_local)

    print(f'Updating Community spec with ipfs cid: {ipfs_cid}')
    update_spec_with_cid(spec_file_path, ipfs_cid)

    print(client.list_communities())
    client.go_to_phase(CeremonyPhase.REGISTERING)

    # charlie has no genesis funds
    print('Faucet is dripping to Charlie...')
    client.faucet([account3], is_faucet = True)

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print('Registering Participants...')
    [client.register_participant(b, cid) for b in accounts]

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print(client.list_participants(cid))
    client.next_phase()

    print('Listing meetups')
    print(client.list_meetups(cid))
    client.next_phase()

    print(f'Performing meetups for cid {cid}')
    perform_meetup(client, cid)

    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print(client.list_attestees(cid))
    client.next_phase()

    print(f'Balances for new community with cid: {cid}.')
    bal = [client.balance(a, cid=cid) for a in accounts]
    [print(f'Account balance for {ab[0]} is {ab[1]}.') for ab in list(zip(accounts, bal))]

    if round(bal[0]) > 0:
        print("tests passed")
    else:
        print("balance is wrong")
        exit(1)


if __name__ == '__main__':
    p = argparse.ArgumentParser(prog='bootstrap-demo-community', parents=[simple_parser()])
    p.add_argument('--ipfs-local', '-l', action='store_true', help="set this option to use the local ipfs daemon")

    args = p.parse_args()

    print(f"Starting script with client '{args.client}' on port {args.port}")

    main(args.ipfs_local, Client(rust_client=args.client, port=args.port))
