#!/usr/bin/env python3
"""
Bootstrap and grow Encointer BOT communities on a *dev* chain or testnet

you may need to install a few packages first
   pip3 install --upgrade pip
   pip3 install randomwords geojson pyproj

then start a node with
   ../target/release/encointer-node-notee --dev --tmp --ws-port 9945 --enable-offchain-indexing true --rpc-methods unsafe

and init and grow a community
   ./bot-community.py --port 9945 init
   ./bot-community.py --port 9945 benchmark
   
on testnet Gesell, run this script once per ceremony phase (after calling `init` first)
   ./bot-community.py --port 9945 run

"""
import argparse
import glob
import os

import geojson

from random_words import RandomWords
from math import floor

from py_client.helpers import purge_prompt, read_cid, write_cid, zip_folder
from py_client.arg_parser import simple_parser
from py_client.client import Client, ExtrinsicFeePaymentImpossible, ExtrinsicWrongPhase, UnknownError, ParticipantAlreadyLinked
from py_client.ipfs import Ipfs, ICONS_PATH
from py_client.communities import populate_locations, generate_community_spec, meta_json

KEYSTORE_PATH = './my_keystore'
NUMBER_OF_LOCATIONS = 100
MAX_POPULATION = 12 * NUMBER_OF_LOCATIONS

def random_community_spec(bootstrappers, ipfs_cid):
    point = geojson.utils.generate_random("Point", boundingBox=[-56, 41, -21, 13])
    locations = populate_locations(point, NUMBER_OF_LOCATIONS)
    print(f'created {len(locations)} random locations around {point}.')

    name = 'bot' + '-'.join(RandomWords().random_words(count=1))
    symbol = name[1:4].upper()
    meta = meta_json(name, symbol, ipfs_cid)
    print(f'CommunityMetadata {meta}')
    return generate_community_spec(meta, locations, bootstrappers)


def init_bootstrappers(client: Client):
    bootstrappers = client.create_accounts(10)
    print('created bootstrappers: ' + ' '.join(bootstrappers))
    client.faucet(bootstrappers)
    client.await_block()
    return bootstrappers


def purge_keystore_prompt():
    purge_prompt(KEYSTORE_PATH, 'accounts')


def init(client: str, port: str, ipfs_local: str, node_url: str):
    # print("ipfs_api_key_in_init_argument", ipfs_api_key)
    client = setLocalOrRemoteChain(client, node_url, port)
    purge_keystore_prompt()

    root_dir = os.path.realpath(ICONS_PATH)
    zipped_folder = zip_folder("icons",root_dir)
    try:
        ipfs_cid = Ipfs.add(zipped_folder, ipfs_local)
    except:
        print("add image to ipfs failed")
    print('initializing community')
    b = init_bootstrappers(client)
    specfile = random_community_spec(b, ipfs_cid)
    print(f'generated community spec: {specfile} first bootstrapper {b[0]}')
    cid = client.new_community(specfile, b[0])
    print(f'created community with cid: {cid}')
    write_cid(cid)


def setLocalOrRemoteChain(client, node_url, port):
    if (node_url == None):
        client = Client(rust_client=client, port=port)
    else:
        client = Client(rust_client=client, node_url='wss://gesell.encointer.org', port=443)
    return client


def register_participants(client: Client, accounts, cid):
    bal = [client.balance(a, cid=cid) for a in accounts]
    total = sum(bal)
    print(f'****** money supply is {total}')
    f = open('bot-stats.csv', 'a')
    f.write(f'{len(accounts)}, {total}\n')
    f.close()
    if total > 0:
        n_newbies = min(floor(len(accounts) / 4.0), MAX_POPULATION - len(accounts))
        print(f'*** adding {n_newbies} newbies')
        if n_newbies > 0:
            newbies = []
            for n in range(0, n_newbies):
                newbies.append(client.new_account())
            client.faucet(newbies)
            client.await_block()
            accounts = client.list_accounts()

    print(f'registering {len(accounts)} participants')
    need_refunding = []
    for p in accounts:
        # print(f'registering {p}')
        try:
            client.register_participant(p, cid)
        except ExtrinsicFeePaymentImpossible:
            need_refunding.append(p)
        except ParticipantAlreadyLinked:
            pass

    if len(need_refunding) > 0:
        print(f'the following accounts are out of funds and will be refunded {need_refunding}')
        client.faucet(need_refunding)

def perform_meetup(client: Client, meetup, cid):
    n = len(meetup)
    print(f'Performing meetup with {n} participants')

    claims = [client.new_claim(p, n, cid) for p in meetup]

    for p_index in range(len(meetup)):
        attestor = meetup[p_index]
        attestees_claims = claims[:p_index] + claims[p_index + 1:]
        client.attest_claims(attestor, attestees_claims)


def run(client: str, port: int, node_url: str):
    client = setLocalOrRemoteChain(client,port,node_url)
    cid = read_cid()
    phase = client.get_phase()
    print(f'phase is {phase}')
    accounts = client.list_accounts()
    print(f'number of known accounts: {len(accounts)}')
    if phase == 'REGISTERING':
        register_participants(client, accounts, cid)
        client.await_block()
    if phase == "ASSIGNING":
        meetups = client.list_meetups(cid);
        meetup_sizes = list(map(lambda x: len(x), meetups))
        print(f'meetups assigned for {sum(meetup_sizes)} participants with sizes: {meetup_sizes}')
    if phase == 'ATTESTING':
        meetups = client.list_meetups(cid)
        print(f'****** Performing {len(meetups)} meetups')
        for meetup in meetups:
            perform_meetup(client, meetup, cid)
        client.await_block()
    return phase


def benchmark(client: str, port: int, node_url: str):
    py_client = setLocalOrRemoteChain(client,node_url,port)
    print('will grow population forever')
    while True:
        phase = run(client, port)
        while phase == py_client.get_phase():
            py_client.await_block()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='bot-community', parents=[simple_parser()])
    subparsers = parser.add_subparsers(dest='subparser', help='sub-command help')
    # Note: the function args' names `client` and `port` must match the cli's args' names.
    # Otherwise, the the values can't be extracted from the `**kwargs`.
    parser_a = subparsers.add_parser('init', help='a help')
    parser_a.add_argument('--ipfs-local', '-l', action='store_true', help="set this option to use the local ipfs daemon")
    parser_b = subparsers.add_parser('run', help='b help')
    parser_c = subparsers.add_parser('benchmark', help='b help')
    kwargs = vars(parser.parse_args())
    try:
        globals()[kwargs.pop('subparser')](**kwargs)
    except KeyError:
        parser.print_help()
