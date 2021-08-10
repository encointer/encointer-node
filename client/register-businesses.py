#!/usr/bin/env python3
import argparse
import glob
import json
import random

from random_words import RandomWords
from wonderwords import RandomSentence

from py_client.arg_parser import simple_parser
from py_client.client import Client
from py_client.ipfs import Ipfs
from py_client.helpers import purge_prompt, read_cid

BUSINESS_PATH = '../test-data/bazaar/businesses'
OFFERINGS_PATH = '../test-data/bazaar/offerings'


def create_businesses(amount: int):
    purge_prompt(BUSINESS_PATH, 'businesses')

    for i in range(amount):
        b = random_business()
        f_name = f'{BUSINESS_PATH}/{b["name"]}_{i}.json'
        print(f'Dumping business {b} to {f_name}')
        with open(f_name, 'w') as outfile:
            json.dump(b, outfile, indent=2)


def create_offerings(community_identifier: str, amount: int):
    purge_prompt(OFFERINGS_PATH, 'offerings')

    for i in range(amount):
        o = random_offering(community_identifier)
        f_name = f'{OFFERINGS_PATH}/{o["name"]}_{i}.json'
        print(f'Dumping offerings {o} to {f_name}')
        with open(f_name, 'w') as outfile:
            json.dump(o, outfile, indent=2)


def upload_files_to_ipfs(paths):
    return [Ipfs.add_recursive(f) for f in paths]


def random_business():
    s = RandomSentence()
    return {
        "name": RandomWords().random_words(count=1)[0],
        "description": s.sentence()
    }


def random_offering(community_identifier):
    return {
        "name": RandomWords().random_words(count=1)[0],
        "price": random.randint(0, 100),
        "community": community_identifier,
        "image_cid": "Defau1tCidThat1s46Characters1nLength1111111111"
    }


def shop_owners():
    return ['//Alice', '//Bob']


if __name__ == '__main__':
    p = argparse.ArgumentParser(prog='bootstrap-demo-community', parents=[simple_parser()])
    args = p.parse_args()

    print(f"Starting script with client '{args.client}' on port {args.port}")
    client = Client(rust_client=args.client, port=args.port)
    shop_owners = shop_owners()
    cid = read_cid()

    create_businesses(2)
    business_ipfs_cids = upload_files_to_ipfs(glob.glob(BUSINESS_PATH + '/*'))
    print(f'Uploaded businesses to ipfs: ipfs_cids: {business_ipfs_cids}')
    for bi in range(len(business_ipfs_cids)):
        # upload with different owners to test rpc `bazaar_getBusinesses`
        c = business_ipfs_cids[bi]
        owner = shop_owners[bi]
        print(f'registering business:')
        print(f'    cid:            {cid}')
        print(f'    owner:          {owner}')
        print(f'    business url:   {c}\n')

        print(client.create_business(owner, cid, c))
        client.await_block()

    create_offerings(cid, 5)
    offerings_ipfs_cids = upload_files_to_ipfs(glob.glob(OFFERINGS_PATH + '/*'))
    print(f'Uploaded offerings to ipfs: ipfs_cids: {offerings_ipfs_cids}')
    for c in offerings_ipfs_cids:
        # always upload to the same owner to test rpc `bazaar_getOfferingsForBusiness`
        owner = shop_owners[0]

        print(f'registering offering:')
        print(f'    cid:            {cid}')
        print(f'    owner:          {owner}')
        print(f'    offering url:   {c}\n')

        print(client.create_offering(shop_owners[0], cid, c))
        client.await_block()
