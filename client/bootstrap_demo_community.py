#!/usr/bin/env python3
"""
Demonstrate the bootstrapping of an Encointer community on a *dev* chain.

start node with
  ../target/release/encointer-node-notee --dev --tmp --ws-port 9945 --enable-offchain-indexing true --rpc-methods unsafe

then run this script
  ./bootstrap_demo_community.py --port 9945

"""

import json
import os
import click

from py_client.client import Client
from py_client.scheduler import CeremonyPhase
from py_client.ipfs import Ipfs, ICONS_PATH

account1 = '//Alice'
account2 = '//Bob'
account3 = '//Charlie'
accounts = [account1, account2, account3]

TEST_DATA_DIR = './test-data/'
TEST_LOCATIONS_MEDITERRANEAN = 'test-locations-mediterranean.json'

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


@click.command()
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('-l', '--ipfs-local', is_flag=True, help='if set, local ipfs node is used.')
@click.option('-s', '--spec-file', default=f'{TEST_DATA_DIR}{TEST_LOCATIONS_MEDITERRANEAN}', help='Specify community spec-file to be registered.')
def main(ipfs_local, client, port, spec_file):
    client = Client(rust_client=client, port=port)
    spec_file_path = spec_file

    cid = client.new_community(spec_file_path)
    if len(cid) > 10:
        print(f'Registered community with cid: {cid}')
    else:
        exit(1)

    print('Uploading icons to ipfs')
    root_dir = os.path.realpath(ICONS_PATH)
    ipfs_cid = Ipfs.add_recursive(root_dir, ipfs_local)

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

    print(f'Registering Participants for Cid: {cid}')
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

    print("Claiming rewards")
    client.claim_reward(account1, cid)
    client.await_block()

    print(f'Balances for new community with cid: {cid}.')
    bal = [client.balance(a, cid=cid) for a in accounts]
    [print(f'Account balance for {ab[0]} is {ab[1]}.') for ab in list(zip(accounts, bal))]

    if round(bal[0]) > 0:
        print("tests passed")
    else:
        print("balance is wrong")
        exit(1)


if __name__ == '__main__':
    main()
