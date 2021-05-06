#!python
import argparse
import geojson

from random_words import RandomWords
from math import floor

from py_client.client import Client
from py_client.communities import populate_locations, generate_community_spec

NUMBER_OF_LOCATIONS = 100
MAX_POPULATION = 12 * NUMBER_OF_LOCATIONS


def random_community_spec(client=Client()):
    point = geojson.utils.generate_random("Point", boundingBox=[-56, 41, -21, 13])
    locations = populate_locations(point, NUMBER_OF_LOCATIONS)
    print("created " + str(len(locations)) + " random locations around " + str(point))

    bootstrappers = [client.new_account() for _ in range(0, 10)]
    print('new bootstrappers: ' + ' '.join(bootstrappers))
    client.faucet(bootstrappers)
    client.await_block()

    name = '#' + '-'.join(RandomWords().random_words(count=1))
    return generate_community_spec(name, locations, bootstrappers)


def init(client=Client()):
    print("initializing community")
    specfile = random_community_spec(client)
    print("generated community spec: ", specfile)
    cid = client.new_community(specfile)
    print("created community with cid: ", cid)
    f = open("cid.txt", "w")
    f.write(cid)
    f.close()


def register_participants(client, accounts, cid):
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


def perform_meetup(client, meetup, cid):
    n = len(meetup)
    print("Performing meetup with " + str(n) + " participants")
    claims = {}
    for p in meetup:
        claims[p] = client.new_claim(p, n, cid)
    for claimant in meetup:
        attestations = []
        for attester in meetup:
            if claimant == attester:
                continue
            # print(claimant + " is attested by " + attester)
            attestations.append(client.sign_claim(attester, claims[claimant]))
        # print("registering attestations for " + claimant)
        client.register_attestations(claimant, attestations)


def run(client=Client()):
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


def benchmark():
    print("will grow population forever")
    client = Client()
    while True:
        run()
        client.await_block()
        client.next_phase()
        client.await_block()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(prog='bot-community')
    subparsers = parser.add_subparsers(dest='subparser', help='sub-command help')
    parser_a = subparsers.add_parser('init', help='a help')
    parser_b = subparsers.add_parser('run', help='b help')
    parser_c = subparsers.add_parser('benchmark', help='b help')

    kwargs = vars(parser.parse_args())
    try:
        globals()[kwargs.pop('subparser')](**kwargs)
    except KeyError:
        parser.print_help()
