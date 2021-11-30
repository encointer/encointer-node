#!/usr/bin/env python3
import glob
import json
import random

import click

from random_words import RandomWords
from wonderwords import RandomSentence

from py_client.ipfs import Ipfs
from py_client.helpers import purge_prompt, read_cid, mkdir_p, set_local_or_remote_chain

BUSINESSES_PATH = './test-data/bazaar/businesses'
OFFERINGS_PATH = './test-data/bazaar/offerings'

ICON_PATH = './test-data/icons/community_icon.png'

global IPFS_LOCAL

@click.command()
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('-l', '--ipfs_local', is_flag=True, help='if set, local ipfs node is used.')
@click.option('-r', '--remote_chain', default=None, help='choose one of the remote chains: gesell.')
def register_businesses_and_offerings(client, port, ipfs_local, remote_chain):
    client = set_local_or_remote_chain(client, port, remote_chain)
    global IPFS_LOCAL
    IPFS_LOCAL = ipfs_local
    owners = shop_owners()

    # As we try to read to the cid here, we must have called `./bootstrap_demo_community.py init` before calling this
    # script
    cid = read_cid()

    create_businesses(2)
    business_ipfs_cids = Ipfs.add_multiple(glob.glob(BUSINESSES_PATH + '/*.json'), ipfs_local)
    print(f'Uploaded businesses to ipfs: ipfs_cids: {business_ipfs_cids}')

    for bi in range(len(business_ipfs_cids)):
        # upload with different owners to test rpc `bazaar_getBusinesses`
        c = business_ipfs_cids[bi]
        owner = owners[bi]
        print(f'registering business:')
        print(f'    cid:            {cid}')
        print(f'    owner:          {owner}')
        print(f'    business url:   {c}\n')

        print(client.create_business(owner, cid, c))
        client.await_block()

    create_offerings(cid, 5)

    offerings_ipfs_cids = Ipfs.add_multiple(glob.glob(OFFERINGS_PATH + '/*.json'), ipfs_local)
    print(f'Uploaded offerings to ipfs: ipfs_cids: {offerings_ipfs_cids}')

    for c in offerings_ipfs_cids:
        # always upload to the same owner to test rpc `bazaar_getOfferingsForBusiness`
        owner = owners[0]

        print(f'registering offering:')
        print(f'    cid:            {cid}')
        print(f'    owner:          {owner}')
        print(f'    offering url:   {c}\n')

        print(client.create_offering(owners[0], cid, c))
        client.await_block()

    # Todo: parse the results and evaluate them. Then we can use this script in integration tests
    print(client.list_businesses(cid))
    print(client.list_offerings(cid))
    print(client.list_offerings_for_business(cid, owners[0]))


def create_businesses(amount: int):
    """
    Create some businesses and dump them to the test-data dir.

    :param amount:
    :return:
    """
    purge_prompt(BUSINESSES_PATH, 'businesses')
    mkdir_p(BUSINESSES_PATH)

    for i in range(amount):
        b = random_business()
        f_name = f'{BUSINESSES_PATH}/{b["name"]}_{i}.json'
        print(f'Dumping business {b} to {f_name}')
        with open(f_name, 'w') as outfile:
            json.dump(b, outfile, indent=2)


def create_offerings(community_identifier: str, amount: int):
    """
    Create some offerings and dump them to the test-data dir.

    :param community_identifier:
    :param amount:
    :return:
    """
    purge_prompt(OFFERINGS_PATH, 'offerings')
    mkdir_p(OFFERINGS_PATH)

    for i in range(amount):
        o = random_offering(community_identifier)
        f_name = f'{OFFERINGS_PATH}/{o["name"]}_{i}.json'
        print(f'Dumping offerings {o} to {f_name}')
        with open(f_name, 'w') as outfile:
            json.dump(o, outfile, indent=2)


def random_business():
    """
        Create a random business.

    Note:   This `Business` format is not definite, but it does not matter for simple testing as we upload only
            the ipfs_cid.
            Later, the Icon should be a user specified one, this is just for testing
    :return:
    """
    print("adding business image to remote: ")
    image_cid = Ipfs.add(ICON_PATH, IPFS_LOCAL)
    s = RandomSentence()
    return {
        "name": RandomWords().random_words(count=1)[0],
        "description": s.sentence(),
        "image_cid": image_cid
    }


def random_offering(community_identifier):
    """
    Create a random offering.

    Note:   This `Offering` format is not definite, but it does not matter for simple testing as we upload only
            the ipfs_cid.
    :param community_identifier:
    :return:
    """
    print("adding offering image to remote: ")
    image_cid = Ipfs.add(ICON_PATH, IPFS_LOCAL)
    return {
        "name": RandomWords().random_words(count=1)[0],
        "price": random.randint(0, 100),
        "community": community_identifier,
        "image_cid": image_cid
    }


def shop_owners():
    """
        Note: Only `//Alice` and `//Bob` have funds. Other accounts need to fauceted as in the other scripts.
    """
    return ['//Alice', '//Bob']


if __name__ == '__main__':
    register_businesses_and_offerings()
