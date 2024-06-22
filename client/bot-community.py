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
import ast
import random
from math import floor

from py_client.communities import random_community_spec, COMMUNITY_SPECS_PATH
from py_client.helpers import purge_prompt, read_cid, write_cid, set_local_or_remote_chain
from py_client.client import Client, ExtrinsicFeePaymentImpossible, ExtrinsicWrongPhase, UnknownError, \
    ParticipantAlreadyLinked
from py_client.ipfs import Ipfs, ASSETS_PATH

KEYSTORE_PATH = './my_keystore'
NUMBER_OF_LOCATIONS = 100
MAX_POPULATION = 10 * NUMBER_OF_LOCATIONS
NUMBER_OF_ENDORSEMENTS_PER_REGISTRATION = 10


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

    root_dir = os.path.realpath(ASSETS_PATH)
    ipfs_cid = "QmDUMMYikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"
    try:
        ipfs_cid = Ipfs.add_recursive(root_dir, ctx['ipfs_local'])
    except:
        print("add image to ipfs failed")
    print('initializing community')
    b = init_bootstrappers(client)
    client.await_block()
    specfile = random_community_spec(b, ipfs_cid, NUMBER_OF_LOCATIONS)
    print(f'generated community spec: {specfile} first bootstrapper {b[0]}')

    while True:
        phase = client.get_phase()
        if phase == 'Registering':
            break
        print(f"waiting for ceremony phase Registering. now is {phase}")
        client.await_block()

    cid = client.new_community(specfile, signer='//Alice')
    print(f'created community with cid: {cid}')
    write_cid(cid)
    client.await_block()
    print(client.list_communities())


@cli.command()
def purge_communities():
    purge_prompt(COMMUNITY_SPECS_PATH, 'communities')


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
    if phase == 'Registering':
        print("all participants claim their potential reward")
        for account in accounts:
            client.claim_reward(account, cid)
        client.await_block(3)

        update_proposal_states(client, accounts[0])
        submit_democracy_proposals(client, cid, accounts[0])

        total_supply = write_current_stats(client, accounts, cid)
        if total_supply > 0:
            init_new_community_members(client, cid, len(accounts))

        # updated account list with new community members
        accounts = client.list_accounts()

        register_participants(client, accounts, cid)
        client.await_block()

    if phase == "Assigning":
        meetups = client.list_meetups(cid)
        meetup_sizes = list(map(lambda x: len(x), meetups))
        print(f'meetups assigned for {sum(meetup_sizes)} participants with sizes: {meetup_sizes}')
        update_proposal_states(client, accounts[0])
        vote_on_proposals(client, cid, accounts)
    if phase == 'Attesting':
        meetups = client.list_meetups(cid)
        update_proposal_states(client, accounts[0])
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
            print("awaiting next phase...")
            py_client.await_block()


@cli.command()
@click.pass_obj
def test(ctx):
    py_client = ctx['client']
    print('will grow population for fixed number of ceremonies')
    for i in range(3 * 2 + 1):
        phase = _execute_current_phase(py_client)
        while phase == py_client.get_phase():
            print("awaiting next phase...")
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
        end = start + endorsement_count

        print(f'bootstrapper {endorser} endorses {endorsement_count} accounts.')

        # print(f'bootstrapper:                       {endorser}\n')
        # print(f'endorses the following accounts:    {endorsees[start:end]}')

        client.endorse_newcomers(cid, endorser, endorsees[start:end])

        start += endorsement_count

    return endorsees


def get_newbie_amount(current_population: int):
    return min(
        # register more than can participate, to test restrictions
        floor(current_population / 1.5),
        MAX_POPULATION - current_population
    )


def write_current_stats(client: Client, accounts, cid):
    bal = [client.balance(a, cid=cid) for a in accounts]

    total = sum(bal)
    print(f'****** money supply is {total}')
    f = open('bot-stats.csv', 'a')
    f.write(f'{len(accounts)}, {round(total)}\n')
    f.close()
    return total


def init_new_community_members(client: Client, cid: str, current_community_size: int):
    """ Initializes new community members based on the `current_community_size` and the amount of endorsements we can
        perform.

        :returns Funded accounts, ready to be registered for a ceremony.
    """
    # transform string to python list
    bootstrappers_with_tickets = ast.literal_eval(client.get_bootstrappers_with_remaining_newbie_tickets(cid))

    print(f'Bootstrappers with remaining newbie tickets {bootstrappers_with_tickets}')

    endorsees = endorse_new_accounts(client, cid, bootstrappers_with_tickets, NUMBER_OF_ENDORSEMENTS_PER_REGISTRATION)

    if len(endorsees) > 0:
        print(f'Awaiting endorsement process \n')
        # We don't need to wait here, but if there are any errors the logs mix with the fauceting, which is confusing.
        client.await_block()
        print(f'Added endorsees to community: {len(endorsees)}')

    newbies = client.create_accounts(get_newbie_amount(current_community_size + len(endorsees)))

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
            try:
                client.register_participant(p, cid)
            except ExtrinsicFeePaymentImpossible:
                print("refunding failed")


def perform_meetup(client: Client, meetup, cid):
    n = len(meetup)
    print(f'Performing meetup with {n} participants')

    for p_index in range(len(meetup)):
        attestor = meetup[p_index]
        attendees = meetup[:p_index] + meetup[p_index + 1:]
        client.attest_attendees(attestor, cid, attendees)


def submit_democracy_proposals(client: Client, cid: str, proposer: str):
    print("submitting new democracy proposals")
    client.submit_update_nominal_income_proposal(proposer, 1.1, cid)


def vote_on_proposals(client: Client, cid: str, voters: list):
    proposals = client.get_proposals()
    for proposal in proposals:
        print(
            f"checking proposal {proposal.id}, state: {proposal.state}, approval: {proposal.approval} turnout: {proposal.turnout}")
        if proposal.state == 'Ongoing':
            choices = ['aye', 'nay']
            target_approval = random.random()
            print(f"setting target approval to {target_approval * 100}%")
            weights = [target_approval, 1 - target_approval]

            for voter in voters:
                reputations = [[t[1], t[0]] for t in client.reputation(voter)]
                if len(reputations) == 0:
                    print(f"no reputations for {voter}. can't vote")
                    continue
                vote = random.choices(choices, weights)[0]
                print(f"voting {vote} on proposal {proposal.id} with {voter} and reputations {reputations}")
                client.vote(voter, proposal.id, vote, reputations)
        client.await_block()


def update_proposal_states(client: Client, who: str):
    proposals = client.get_proposals()
    for proposal in proposals:
        print(
            f"checking proposal {proposal.id}, state: {proposal.state}, approval: {proposal.approval} turnout: {proposal.turnout}")
        if proposal.state in ['Ongoing', 'Confirming']:
            print(f"updating proposal {proposal.id}")
            client.update_proposal_state(who, proposal.id)


if __name__ == '__main__':
    cli(obj={})
