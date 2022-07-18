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
from py_client.ipfs import Ipfs, ASSETS_PATH

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
        spec['community']['meta']['assets'] = cid
        print(spec)
        # go to beginning of the file to overwrite
        spec_json.seek(0)
        json.dump(spec, spec_json, indent=2)
        spec_json.truncate()


def create_community(client, spec_file_path, ipfs_local):
    cid = client.new_community(spec_file_path)
    if len(cid) > 10:
        print(f'Registered community with cid: {cid}')
    else:
        exit(1)

    print('Uploading assets to ipfs')
    root_dir = os.path.realpath(ASSETS_PATH)
    ipfs_cid = Ipfs.add_recursive(root_dir, ipfs_local)

    print(f'Updating Community spec with ipfs cid: {ipfs_cid}')
    update_spec_with_cid(spec_file_path, ipfs_cid)

    return cid


def register_participants_and_perform_meetup(client, cid, accounts):
    print(client.list_communities())
    client.go_to_phase(CeremonyPhase.Registering)

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


def faucet(client, cid):
    # charlie has no genesis funds
    print('Faucet is dripping to Charlie...')
    client.faucet([account3], is_faucet=True)

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)


def fee_payment_transfers(client, cid):
    print(f'Transfering 0.5CC from //Alice to //Eve')
    client.transfer(cid, '//Alice', '//Eve', '0.5', pay_fees_in_cc=False)

    print(f'Transfering all CC from //Eve to //Ferdie')
    client.transfer_all(cid, '//Eve', '//Ferdie', pay_fees_in_cc=True)
    if client.balance('//Eve', cid=cid) > 0 or client.balance('//Ferdie', cid=cid) == 0:
        print("transfer_all failed")
        exit(1)


def claim_rewards(client, cid, account):
    print("Claiming rewards")
    client.claim_reward(account, cid)
    client.await_block(3)


def test_reputation_caching(client, cid, account):
    register_participants_and_perform_meetup(client, cid, accounts)
    client.next_phase()
    client.await_block(1)
    # query reputation to set the cache in the same phase as claiming rewards
    # so we would have a valid cache value, but the cache should be invalidated
    # anyways because of the dirty bit
    client.reputation(account1)
    claim_rewards(client, cid, account1)

    # check if the reputation cache was updated
    rep = client.reputation(account1)
    print(rep)
    if ('1', ' sqm1v79dF6b', 'VerifiedLinked') not in rep or ('2', ' sqm1v79dF6b', 'VerifiedUnlinked') not in rep:
        print("wrong reputation")
        exit(1)

    register_participants_and_perform_meetup(client, cid, accounts)
    client.next_phase()
    client.await_block(1)
    # here we dont query the reputation, so the last cache value was set in a previous phase
    # this tests if reputations are updated on phase change
    # client.reputation(account1)

    claim_rewards(client, cid, account1)

    # check if the reputation cache was updated
    rep = client.reputation(account1)
    # here the reputation should be correctly read from the cache
    rep = client.reputation(account1)
    print(rep)
    if ('1', ' sqm1v79dF6b', 'VerifiedLinked') not in rep or ('2', ' sqm1v79dF6b', 'VerifiedLinked') not in rep or ('3', ' sqm1v79dF6b', 'VerifiedUnlinked') not in rep:
        print("wrong reputation")
        exit(1)

    # test if reputation cache is invalidated after registration
    print(f'Registering Participants for Cid: {cid}')
    [client.register_participant(b, cid) for b in accounts]

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    rep = client.reputation(account1)
    print(rep)
    # after the registration the third reputation should now be linked
    if ('3', ' sqm1v79dF6b', 'VerifiedLinked') not in rep:
        print("reputation not linked")
        exit(1)

    client.next_phase()
    client.next_phase()
    client.next_phase()
    client.await_block(1)


@click.command()
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('-l', '--ipfs-local', is_flag=True, help='if set, local ipfs node is used.')
@click.option('-s', '--spec-file', default=f'{TEST_DATA_DIR}{TEST_LOCATIONS_MEDITERRANEAN}', help='Specify community spec-file to be registered.')
def main(ipfs_local, client, port, spec_file):
    client = Client(rust_client=client, port=port)
    cid = create_community(client, spec_file, ipfs_local)
    faucet(client, cid)

    register_participants_and_perform_meetup(client, cid, accounts)
    client.next_phase()
    client.await_block(1)
    claim_rewards(client, cid, account1)

    print(f'Balances for new community with cid: {cid}.')
    bal = [client.balance(a, cid=cid) for a in accounts]
    [print(f'Account balance for {ab[0]} is {ab[1]}.') for ab in list(zip(accounts, bal))]

    if not round(bal[0]) > 0:
        print("balance is wrong")
        exit(1)
    rep = client.reputation(account1)
    print(rep)
    if not len(rep) > 0:
        print("no reputation gained")
        exit(1)

    fee_payment_transfers(client, cid)

    test_reputation_caching(client, cid, accounts)

    print("tests passed")


if __name__ == '__main__':
    main()
