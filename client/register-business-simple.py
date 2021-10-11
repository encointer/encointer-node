#!/usr/bin/env python3
# import argparse
# from py_client.arg_parser import simple_parser

import json
import glob
from py_client.client import Client
from py_client.ipfs import Ipfs
from py_client.helpers import read_cid

ICON_PATH = '../test-data/icons/community_icon.png'
BUSINESSES_PATH = '../test-data/bazaar/businesses'
OFFERINGS_PATH = '../test-data/bazaar/offerings'

# Before running this script, make sure, that a community is registered on the chain (for example by running bot-community.py init)

def create_business(name: str, description: str, ipfs_local):
    """
    Create a business and register it on chain

    :param name: name of the business
    :param description: about the business
    :return:
    """
    print(f"creating business {name} with description: {description}")

    print("ipfs_local",ipfs_local)
    try:
        image_cid = Ipfs.add(ICON_PATH, ipfs_local)
    except:
        print("failed to add image to ipfs")
    return {
        "name": name,
        "description": description,
        "image_cid": image_cid
    }



def register_business(name: str, description: str, owner, chain_local, ipfs_l):
    if chain_local:
        print("registering on local chain")
        client = Client()
    else:
        print("registering on remote chain")
        client = Client(node_url='wss://gesell.encointer.org')

    cid = read_cid()
    ipfs_local = False
    if ipfs_l == 'y': ipfs_local = True
    business_json = create_business(name, description, ipfs_local)
    f_name = f'{BUSINESSES_PATH}/{business_json["name"]}.json'
    print(f'Dumping business {business_json} to {f_name}')
    with open(f_name, 'w') as outfile:
        json.dump(business_json, outfile, indent=2)
    print("f_name, the business_dump_path: ", f_name)
    ipfs_cid = Ipfs.add(f_name, ipfs_local)
    print(f'Uploaded business to ipfs: ipfs_cid: {ipfs_cid}')
    print(f"registering business on chain for cid {cid}")
    print(client.create_business(owner, cid, ipfs_cid))
    client.await_block()



def create_offering(name: str, price: int, community_identifier, ipfs_local):
    """
    Create an offering.

    Note:   This `Offering` format is not definite, but it does not matter for simple testing as we upload only
            the ipfs_cid.
    :param community_identifier:
    :return:
    """
    try:
        print("adding offering image to remote: ")
        image_cid = Ipfs.add(ICON_PATH, ipfs_local)
    except:
        print("failed to add image to ipfs")
    return {
        "name": name,
        "price": price,
        "community": community_identifier,
        "image_cid": image_cid
    }



def register_offering(name: str, price: int, owner, chain_local, ipfs_l):
    if chain_local:
        print("registering on local chain")
        client = Client()
    else:
        print("registering on remote chain")
        client = Client(node_url='wss://gesell.encointer.org')

    cid = read_cid()
    ipfs_local = False
    if ipfs_l == 'y':
        ipfs_local = True

    offering_json = create_offering(name, price, cid, ipfs_local)

    f_name = f'{OFFERINGS_PATH}/{offering_json["name"]}.json'
    print(f'Dumping offerings {offering_json} to {f_name}')
    with open(f_name, 'w') as outfile:
        json.dump(offering_json, outfile, indent=2)

    ipfs_cid = Ipfs.add(f_name, ipfs_local)
    print(f'Uploaded offering to ipfs: ipfs_cid: {ipfs_cid}')
    print(f"registering offering on chain for cid {cid}")
    print(client.create_offering(owner, cid, ipfs_cid))
    client.await_block()

if __name__ == '__main__':
    b_name = input("Enter a name for your business:")
    print(b_name)
    b_descr = input("Enter a description for your business:")
    print(b_descr)
    ipfs_local = input("Do you want to use local ipfs? [y, n]")
    chain_local = input("Do you want to use local chain? [y, n]")
    owner = input("Enter name of the owner:")

    register_business(b_name,b_descr,owner,chain_local,ipfs_local)
    print(f"business {b_name} is being registered on node")
    offering = input("Do you want to register an offering? [y, n]")
    if(offering == 'y'):
        o_name = input("Enter a name for your offering:")
        o_price = input("Enter a price for your offering:")
        register_offering(o_name, o_price, owner,chain_local,ipfs_local)
        print(f"offering {o_name} is being registered on node")

    # parser = argparse.ArgumentParser(prog='register-business-simple', parents=[simple_parser()])
    # subparsers = parser.add_subparsers(dest='subparser', help='sub-command help')
    # parser_a = subparsers.add_parser('register_business', help='a help')
    # parser_a.add_argument('--ipfs-local', '-l', action='store_true', help="set this option to use the local ipfs daemon")
    # parser_a.add_argument('--chain-local', '-c', action='store_true', help="set this option to use the local ipfs daemon")
    # args = parser_a.parse_args()
    # kwargs = vars(parser.parse_args())
    # try:
    #     globals()[kwargs.pop('subparser')](**kwargs)
    # except KeyError:
    #     parser.print_help()
