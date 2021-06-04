#!/usr/bin/env python3
import argparse

import geojson

from random_words import RandomWords
from math import floor

from py_client.arg_parser import simple_parser
from py_client.client import Client
from py_client.ipfs import Ipfs, ICONS_PATH
from py_client.communities import populate_locations, generate_community_spec, meta_json

NUMBER_OF_LOCATIONS = 100
MAX_POPULATION = 12 * NUMBER_OF_LOCATIONS


def random_community_spec(bootstrappers, ipfs_cid):
    point = geojson.utils.generate_random("Point", boundingBox=[-56, 41, -21, 13])
    locations = populate_locations(point, NUMBER_OF_LOCATIONS)
    print("created " + str(len(locations)) + " random locations around " + str(point))

    name = '#' + '-'.join(RandomWords().random_words(count=1))
    symbol = name[1:4].upper()
    meta = meta_json(name, symbol, ipfs_cid)

    return generate_community_spec(meta, locations, bootstrappers)


def init_bootstrappers(client=Client()):
    bootstrappers = client.create_accounts(10)
    print('created bootstrappers: ' + ' '.join(bootstrappers))
    client.faucet(bootstrappers)
    client.await_block()
    return bootstrappers


def init(client: str, port: str):
    client = Client(rust_client=client, port=port)
    ipfs_cid = Ipfs.add_recursive(ICONS_PATH)
    print("initializing community")
    b = init_bootstrappers(client)
    specfile = random_community_spec(b, ipfs_cid)
    print("generated community spec: ", specfile)
    cid = client.new_community(specfile)
    print("created community with cid: ", cid)
    f = open("cid.txt", "w")
    f.write(cid)
    f.close()


def register_participants(client: Client, accounts, cid):
    bal = [client.balance(a, cid=cid) for a in accounts]
    total = sum(bal)
    print("****** money supply is " + str(total))
    f = open("bot-stats.csv", "a")
    f.write(str(len(accounts)) + ", " + str(total) + "\n")
    f.close()
    if total > 0:
        n_newbies = min(floor(len(accounts) / 4.0), MAX_POPULATION - len(accounts))
        print("*** adding " + str(n_newbies) + " newbies")
        if n_newbies > 0:
            newbies = []
            for n in range(0, n_newbies):
                newbies.append(client.new_account())
            client.faucet(newbies)
            client.await_block()
            accounts = client.list_accounts()

    print("registering " + str(len(accounts)) + " participants")
    for p in accounts:
        # print("registering " + p)
        client.register_participant(p, cid)


def perform_meetup(client: Client, meetup, cid):
    n = len(meetup)
    print("Performing meetup with " + str(n) + " participants")

    claims = [client.new_claim(p, n, cid) for p in meetup]

    for p_index in range(len(meetup)):
        attestor = meetup[p_index]
        attestees_claims = claims[:p_index] + claims[p_index + 1:]
        client.attest_claims(attestor, attestees_claims)


def run(client: str, port: int):
    client = Client(rust_client=client, port=port)
    f = open("cid.txt", "r")
    cid = f.read()
    print("cid is " + cid)
    phase = client.get_phase()
    print("phase is " + phase)
    accounts = client.list_accounts()
    print("number of known accounts: " + str(len(accounts)))
    if phase == 'REGISTERING':
        register_participants(client, accounts, cid)
        client.await_block()
    if phase == 'ATTESTING':
        meetups = client.list_meetups(cid)
        print("****** Performing " + str(len(meetups)) + " meetups")
        for meetup in meetups:
            perform_meetup(client, meetup, cid)
        client.await_block()


def benchmark(client: str, port: int):
    py_client = Client(rust_client=client, port=port)
    print("will grow population forever")
    while True:
        run(client, port)
        py_client.await_block()
#        py_client.next_phase()
#        py_client.await_block()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='bot-community', parents=[simple_parser()])
    subparsers = parser.add_subparsers(dest='subparser', help='sub-command help')

    # Note: the function args' names `client` and `port` must match the cli's args' names.
    # Otherwise, the the values can't be extracted from the `**kwargs`.
    parser_a = subparsers.add_parser('init', help='a help')
    parser_b = subparsers.add_parser('run', help='b help')
    parser_c = subparsers.add_parser('benchmark', help='b help')

    kwargs = vars(parser.parse_args())
    try:
        globals()[kwargs.pop('subparser')](**kwargs)
    except KeyError:
        parser.print_help()
