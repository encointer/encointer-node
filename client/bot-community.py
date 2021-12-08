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
   
on testnet Gesell, execute the current ceremony phase (it does not advance the phase).
   ./bot-community.py --port 9945 execute-current-phase

"""
import os

import click
import ast

from math import floor

from py_client.communities import random_community_spec
from py_client.helpers import purge_prompt, read_cid, write_cid, zip_folder, set_local_or_remote_chain
from py_client.client import Client, ExtrinsicFeePaymentImpossible, ExtrinsicWrongPhase, UnknownError, \
    ParticipantAlreadyLinked
from py_client.ipfs import Ipfs, ICONS_PATH

KEYSTORE_PATH = './my_keystore'
NUMBER_OF_LOCATIONS = 100
MAX_POPULATION = 12 * NUMBER_OF_LOCATIONS
NUMBER_OF_ENDORSEMENTS_PER_REGISTRATION = 101


@click.group()
@click.option('--client', default='../target/release/encointer-client-notee',
              help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('-l', '--ipfs_local', is_flag=True, help='if set, local ipfs node is used.')
@click.option('-r', '--remote_chain', default=None, help='choose one of the remote chains: gesell.')
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
    specfile = random_community_spec(b, ipfs_cid, NUMBER_OF_LOCATIONS)
    print(f'generated community spec: {specfile} first bootstrapper {b[0]}')
    cid = client.new_community(specfile, b[0])
    print(f'created community with cid: {cid}')
    write_cid(cid)
    client.await_block()


@cli.command()
@click.pass_obj
def execute_current_phase(ctx):
    return _execute_current_phase(ctx['client'])


def _execute_current_phase(client: Client):
    client = client
    cid = read_cid()
    phase = client.get_phase()
    print(f'phase is {phase}')
    accounts = client.list_accounts()
    print(f'number of known accounts: {len(accounts)}')
    if phase == 'REGISTERING':
        write_current_stats(client, accounts, cid)

        init_new_community_members(client, cid, len(accounts))

        # updated account list with new community members
        accounts = client.list_accounts()

        register_participants(client, accounts, cid)
        client.await_block()

    if phase == "ASSIGNING":
        meetups = client.list_meetups(cid)
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
        phase = _execute_current_phase(py_client)
        while phase == py_client.get_phase():
            py_client.await_block()


def init_bootstrappers(client: Client):
    bootstrappers = client.create_accounts(10)
    print('created bootstrappers: ' + ' '.join(bootstrappers))
    client.faucet(bootstrappers)
    client.await_block()
    return bootstrappers


def purge_keystore_prompt():
    purge_prompt(KEYSTORE_PATH, 'accounts')


def get_endorsement_allocation(bootstrappers_and_tickets, endorsee_count: int):
    """ Returns an endorsement allocation based on the available newbie tickets of the bootstrappers and the total amount
        of endorsements we want to execute.

        Also returns the amount of possible endorsements.
    """
    endorsers = []
    e_count = endorsee_count
    effective_endorsements = 0

    for bootstrapper, remaining_tickets in bootstrappers_and_tickets:
        tickets = min(remaining_tickets, e_count)

        if tickets > 0:
            endorsers.append((bootstrapper, tickets))
            effective_endorsements += tickets

        e_count -= tickets

        if e_count <= 0:
            break

    return (endorsers, effective_endorsements)


def endorse_new_accounts(client: Client, cid: str, bootstrappers_and_tickets, endorsee_count: int):
    """ Endorse some new accounts. They are not fauceted.

        Tries to endorse up to `endorsee_count` new accounts, but will do fewer if there are not enough bootstrapper
        newbie tickets left.
    """
    (endorsers_and_tickets, total_endorsements) = get_endorsement_allocation(bootstrappers_and_tickets, endorsee_count)

    if total_endorsements == 0:
        print("Can't endorse anymore, all tickets have been spent.")
        return []

    endorsees = client.create_accounts(total_endorsements)

    start = 0
    for endorser, endorsement_count in endorsers_and_tickets:
        # execute endorsements per bootstrapper
        end = start+endorsement_count

        print(f'bootstrapper {endorser} endorses {endorsement_count} accounts.')

        # print(f'bootstrapper:                       {endorser}\n')
        # print(f'endorses the following accounts:    {endorsees[start:end]}')

        client.endorse_newcomers(cid, endorser, endorsees[start:end])

        start += endorsement_count

    return endorsees


def get_newbie_amount(current_population: int):
    return min(
        floor(current_population / 4.0),
        MAX_POPULATION - current_population
    )


def write_current_stats(client: Client, accounts, cid):
    bal = [client.balance(a, cid=cid) for a in accounts]

    total = sum(bal)
    print(f'****** money supply is {total}')
    f = open('bot-stats.csv', 'a')
    f.write(f'{len(accounts)}, {total}\n')
    f.close()


def init_new_community_members(client: Client, cid: str, current_community_size: int):
    """ Initializes new community members based on the `current_community_size` and the amount of endorsements we can
        perform.

        :returns Funded accounts, ready to be registered for a ceremony.
    """
    # transform string to python list
    bootstrappers_with_tickets = ast.literal_eval(client.get_bootstrappers_with_remaining_newbie_tickets(cid))

    print(f'Bootstrappers with remaining newbie tickets {bootstrappers_with_tickets}')

    endorsees = endorse_new_accounts(client, cid, bootstrappers_with_tickets, NUMBER_OF_ENDORSEMENTS_PER_REGISTRATION)

    print(f'Awaiting endorsement process \n')
    # We don't need to wait here, but if there are any errors the logs mix with the fauceting, which is confusing.
    client.await_block()

    print(f'Added endorsees to community: {len(endorsees)}')

    newbies = client.create_accounts(get_newbie_amount(current_community_size))

    print(f'Add newbies to community {len(newbies)}')

    new_members = newbies + endorsees

    client.faucet(new_members)
    client.await_block()

    print(f'Fauceted new community members {len(new_members)}')

    return new_members


def register_participants(client: Client, accounts, cid):
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

        client.await_block()

        for p in need_refunding:
            client.register_participant(p, cid)


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
