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
import os

import click
import geojson

from random_words import RandomWords
from math import floor

from py_client.helpers import purge_prompt, read_cid, write_cid, zip_folder, set_local_or_remote_chain
from py_client.client import Client, ExtrinsicFeePaymentImpossible, ExtrinsicWrongPhase, UnknownError, ParticipantAlreadyLinked
from py_client.ipfs import Ipfs, ICONS_PATH
from py_client.communities import populate_locations, generate_community_spec, meta_json

KEYSTORE_PATH = './my_keystore'
NUMBER_OF_LOCATIONS = 100
MAX_POPULATION = 12 * NUMBER_OF_LOCATIONS


@click.group()
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('-l', '--ipfs_local', is_flag=True, help='if set, local ipfs node is used')
@click.option('-r', '--remote_chain', default=None, help='choose one of the remote chains: gesell, gesell-dot, gesell-ksm, cantillon-dot, cantillon-ksm')
@click.pass_context
def cli(ctx, client, port, ipfs_local, remote_chain):
    ctx.ensure_object(dict)
    cl = set_local_or_remote_chain(client, port, remote_chain)
    ctx.obj['client'] = cl
    ctx.obj['port'] = port
    ctx.obj['ipfs_local'] = ipfs_local
    ctx.obj['remote_chain'] = remote_chain


@cli.command()
@click.pass_obj
def init(ctx):
    client = ctx['client']
    purge_keystore_prompt()

    root_dir = os.path.realpath(ICONS_PATH)
    zipped_folder = zip_folder("icons", root_dir)
    try:
        ipfs_cid = Ipfs.add(zipped_folder, ctx['ipfs_local'])
    except:
        print("add image to ipfs failed")
    print('initializing community')
    b = init_bootstrappers(client)
    specfile = random_community_spec(b, ipfs_cid)
    print(f'generated community spec: {specfile} first bootstrapper {b[0]}')
    cid = client.new_community(specfile, b[0])
    print(f'created community with cid: {cid}')
    write_cid(cid)


@cli.command()
@click.pass_obj
def run(ctx):
    return execute_current_phase(ctx['client'])


def execute_current_phase(client: Client):
    client = client
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


@cli.command()
@click.pass_obj
def benchmark(ctx):
    py_client = ctx['client']
    print('will grow population forever')
    while True:
        phase = execute_current_phase(py_client)
        while phase == py_client.get_phase():
            py_client.await_block()


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


if __name__ == '__main__':
    cli(obj={})
