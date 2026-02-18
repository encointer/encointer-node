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
   ./bot-community.py --port 9945 simulate --ceremonies 7

on testnet Gesell, execute the current ceremony phase (it does not advance the phase).
   ./bot-community.py --port 9945 execute-current-phase


NOTE: There are a few extrinsic errors, which are (sometimes) ok to be thrown:
    * Only ok in the first ceremony:
        Module(ModuleError { index: 61, error: 1, message: None }) DispatchInfo { weight: 10000, class: DispatchClass::Normal, pays_fee: Pays::Yes }
        Meaning: Tried to claim rewards when account was not registered. This happens in the first ceremony because no previous meetup took place.

    * Always Ok:
        Module(ModuleError { index: 61, error: 21, message: None }) DispatchInfo { weight: 10000, class: DispatchClass::Normal, pays_fee: Pays::Yes }
        Meaning: Reward was already claimed. This happens because only one participant needs to claim the reward for the whole meetup, afterwards
        above error is thrown.

"""
import os

import click

from py_client.communities import random_community_spec, COMMUNITY_SPECS_PATH
from py_client.helpers import purge_prompt, read_cid, write_cid, set_local_or_remote_chain
from py_client.client import Client
from py_client.ipfs import Ipfs, ASSETS_PATH
from py_client.agent_pool import AgentPool
from py_client.simulation_log import SimulationLog

KEYSTORE_PATH = './my_keystore'


@click.group()
@click.option('--client', default='../target/release/encointer-client-notee',
              help='Client binary to communicate with the chain.')
@click.option('-p', '--port', default='9944', help='ws-port of the chain.')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain, or `gesell` alternatively.')
@click.option('-l', '--ipfs_local', is_flag=True, help='if set, local ipfs node is used.')
@click.option('-f', '--faucet_url', default='http://localhost:5000/api',
              help='url for the faucet')
@click.option('-w', '--wrap-call', default="none", help='wrap the call, values: none|sudo|collective')
@click.option('-b', '--batch-size', default=100, help='batch size of the addLocation call')
@click.option('-n', '--number-of-locations', default=100, help='number of locations to generate for the bot-community')
@click.option('--waiting-blocks', default=1, help='Waiting time between steps')
@click.pass_context
def cli(ctx, client, port, ipfs_local, url, faucet_url, wrap_call, batch_size, number_of_locations, waiting_blocks):
    ctx.ensure_object(dict)
    cl = set_local_or_remote_chain(client, port, url)
    ctx.obj['client'] = cl
    ctx.obj['port'] = port
    ctx.obj['ipfs_local'] = ipfs_local
    ctx.obj['url'] = url
    ctx.obj['faucet_url'] = faucet_url
    ctx.obj['wrap_call'] = wrap_call
    ctx.obj['batch_size'] = batch_size
    ctx.obj['number_of_locations'] = number_of_locations
    ctx.obj['max_population'] = number_of_locations * 10
    ctx.obj['waiting_blocks'] = waiting_blocks


@cli.command()
@click.pass_obj
def init(ctx):
    client = ctx['client']
    faucet_url = ctx['faucet_url']
    wrap_call = ctx['wrap_call']
    batch_size = ctx['batch_size']
    number_of_locations = ctx['number_of_locations']
    waiting_blocks = ctx['waiting_blocks']
    purge_keystore_prompt()

    root_dir = os.path.realpath(ASSETS_PATH)
    ipfs_cid = "QmDUMMYikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"
    try:
        ipfs_cid = Ipfs.add_recursive(root_dir, ctx['ipfs_local'])
    except:
        print("add image to ipfs failed")
    print('initializing community')

    pool = AgentPool(client, cid='', faucet_url=faucet_url,
                     max_population=ctx['max_population'], waiting_blocks=waiting_blocks)
    b = pool.bootstrap(10)
    client.await_block(waiting_blocks)
    specfile = random_community_spec(b, ipfs_cid, number_of_locations)
    print(f'generated community spec: {specfile} first bootstrapper {b[0]}')

    while True:
        phase = client.get_phase()
        if phase == 'Registering':
            break
        print(f"waiting for ceremony phase Registering. now is {phase}")
        client.await_block(waiting_blocks)

    cid = client.new_community(specfile, signer='//Alice', wrap_call=wrap_call, batch_size=batch_size)
    print(f'created community with cid: {cid}')
    write_cid(cid)
    client.await_block(waiting_blocks)
    print(client.list_communities())

    # Clear stats file for fresh run
    open('bot-stats.csv', 'w').close()


@cli.command()
def purge_communities():
    purge_prompt(COMMUNITY_SPECS_PATH, 'communities')


@cli.command()
@click.pass_obj
def execute_current_phase(ctx):
    """Execute work for the current ceremony phase, then exit. For Gesell cronjob use."""
    client = ctx['client']
    cid = read_cid()
    pool = AgentPool(client, cid=cid, faucet_url=ctx['faucet_url'],
                     max_population=ctx['max_population'], waiting_blocks=ctx['waiting_blocks'])
    pool.load_agents()

    phase = client.get_phase()
    cindex = client.get_cindex()
    print(f'🕑 phase is {phase} and ceremony index is {cindex}')

    if phase == 'Registering':
        pool.execute_registering()
        pool.run_auxiliary_features(cindex % 7 or 7)
    elif phase == 'Assigning':
        pool.execute_assigning()
    elif phase == 'Attesting':
        pool.execute_attesting()

    pool.write_stats()
    return phase


@cli.command()
@click.option('--ceremonies', default=7, help='Number of ceremonies to simulate. 0 for infinite.')
@click.option('--assert-invariants/--no-assert-invariants', default=True,
              help='Run per-ceremony assertions (CI mode).')
@click.pass_obj
def simulate(ctx, ceremonies, assert_invariants):
    """Run N ceremonies with self-advancing phases. Replaces old test/benchmark commands."""
    client = ctx['client']
    cid = read_cid()
    waiting_blocks = ctx['waiting_blocks']

    log = SimulationLog('bot-community-log.txt')
    client.log = log

    pool = AgentPool(client, cid=cid, faucet_url=ctx['faucet_url'],
                     max_population=ctx['max_population'], waiting_blocks=waiting_blocks)
    pool.load_agents()

    infinite = ceremonies == 0
    target = ceremonies if not infinite else float('inf')
    cindex = 0

    print(f'🚀 Starting simulation: {"infinite" if infinite else ceremonies} ceremonies')

    while cindex < target:
        cindex += 1
        log.ceremony(cindex)
        print(f'\n{"="*60}')
        print(f'🔄 Ceremony {cindex}')
        print(f'{"="*60}')

        # Registering phase
        phase = client.get_phase()
        if phase != 'Registering':
            print(f"⚠ Expected Registering, got {phase}. Advancing...")
            while client.get_phase() != 'Registering':
                client.next_phase()
                client.await_block(waiting_blocks)

        log.phase('Registering')
        print(f'\n📋 Phase: Registering')
        pool.execute_registering()

        # Advance to Assigning
        client.next_phase()
        client.await_block(waiting_blocks)

        log.phase('Assigning')
        print(f'\n📋 Phase: Assigning')
        pool.execute_assigning()

        # Advance to Attesting
        client.next_phase()
        client.await_block(waiting_blocks)

        log.phase('Attesting')
        print(f'\n📋 Phase: Attesting')
        pool.execute_attesting()

        # Advance back to Registering
        client.next_phase()
        client.await_block(waiting_blocks)

        # Run auxiliary features for this ceremony
        log.phase('Auxiliary Features')
        pool.run_auxiliary_features(cindex)

        pool.write_ceremony_summary(cindex)

        if assert_invariants:
            pool.assert_invariants(cindex)

        print(f'\n✅ Ceremony {cindex} complete')

    log.close()
    pool.write_stats()
    print(f'\n🏁 Simulation complete: {cindex} ceremonies')
    print(f'📊 Stats written to bot-stats.csv')

    if assert_invariants:
        print(f'🔬 All assertions passed')


def purge_keystore_prompt():
    purge_prompt(KEYSTORE_PATH, 'accounts')


if __name__ == '__main__':
    cli(obj={})
