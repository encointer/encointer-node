#!/usr/bin/env python3
"""
Demonstrate the bootstrapping of an Encointer community on a *dev* chain.

start node with
  ../target/release/encointer-node-notee --dev --tmp --ws-port 9945 --enable-offchain-indexing true --rpc-methods unsafe

or start parachain with  
then run this script
  ./bootstrap_demo_community.py --port 9945

"""
import click
from py_client.client import Client
from lib import *

@click.command()
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain.')
@click.option('-p', '--port', default='9944', help='ws-port of the chain.')
@click.option('-l', '--ipfs-local', is_flag=True, help='if set, local ipfs node is used.')
@click.option('-s', '--spec-file', default=f'{TEST_DATA_DIR}{TEST_LOCATIONS_MEDITERRANEAN}', help='Specify community spec-file to be registered.')
def main(ipfs_local, client, url, port, spec_file):
    client = Client(rust_client=client, node_url=url, port=port)
    cid = create_community(client, spec_file, ipfs_local)


    newbie = client.create_accounts(1)[0]
    faucet(client, cid, [account3, newbie])

    register_participants_and_perform_meetup(client, cid, accounts)

    balance = client.balance(account1)

    print("Claiming early rewards")
    claim_rewards(client, cid, account1)

    if(not balance == client.balance(account1)):
        print("claim_reward fees were not refunded if paid in native currency")
        exit(1)

    client.next_phase()
    client.await_block(1)

    print(f"Community {cid} successfully bootstrapped")

if __name__ == '__main__':
    main()
