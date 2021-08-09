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
    purge_business_prompt()

    for i in range(amount):
        b = random_business()
        f_name = f'{BUSINESS_PATH}/{b["name"]}_{i}.json'
        print(f'Dumping business {b} to file')
        with open(f_name, 'w') as outfile:
            json.dump(b, outfile, indent=2)


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
        "name": RandomWords().random_words(count=1),
        "price": random.randint(0, 100),
        "community": community_identifier
    }


def purge_business_prompt():
    purge_prompt(BUSINESS_PATH, 'businesses')


def purge_offerings_prompt():
    purge_prompt(BUSINESS_PATH, 'offerings')


if __name__ == '__main__':
    p = argparse.ArgumentParser(prog='bootstrap-demo-community', parents=[simple_parser()])
    args = p.parse_args()

    print(f"Starting script with client '{args.client}' on port {args.port}")

    client = Client(rust_client=args.client, port=args.port)

    create_businesses(5)
    cids = upload_files_to_ipfs(glob.glob(BUSINESS_PATH + '/*'))
    for c in cids:
        print(f'uploaded business to ipfs. cid: {c}')
        cid = read_cid()
        print(f'registering business on-chain')
        client.create_business('//Alice', cid, c)
