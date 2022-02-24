#!/usr/bin/env python3

import json
from py_client.client import Client
from py_client.ipfs import Ipfs
from py_client.helpers import read_cid
import json
import click
import tkinter as tk
from tkinter import filedialog
import os

BUSINESSES_PATH = './test-data/bazaar/'

# Before running this script, make sure, that a community is registered on the chain (for example by running bot-community.py init)

@click.command()
@click.option('--cid', default='', help='the community identifier of the community your business belongs to (11 digits)')
@click.option('-l', '--ipfs_local', is_flag=True, help='if set, local ipfs node is used.')
def create_business(cid: str, ipfs_local):
    """
    Register a business on chain

    :param name: path to LocalBusiness.json with all infos specified in https://github.com/encointer/pallets/blob/master/bazaar/README.md
    :return:
    """

    root = tk.Tk()
    root.withdraw()

    bizTitle = 'Select your business json file'
    bizFileRead = filedialog.askopenfile(mode='r', title=bizTitle)
    # should we handle trailling commma of last element? its in general not allowed in python
    businessPyObject = json.load(bizFileRead)
    print(businessPyObject)

    bizImage = 'Select your business image'
    bizImageFile = filedialog.askopenfile(mode='r', title=bizImage)

    if bizImageFile:
        logo_path = os.path.abspath(bizImageFile.name)
        try:
            image_cid = Ipfs.add(logo_path, ipfs_local)
            print("image_cid is:", image_cid)
        except:
            print("failed to add image to ipfs")

    bizImageFile.close()

    businessPyObject['logo'] = image_cid
    bizFileRead.close()
    # handle read & write in one operation?
    bizFileWrite = open(os.path.abspath(bizFileRead.name), 'w')

    json.dump(businessPyObject, bizFileWrite, indent=2)
    bizFileWrite.close()
    print(type(businessPyObject))
    print(businessPyObject)
    return {
        "name": businessPyObject['name'],
        "description": businessPyObject['description'],
        "image_cid": businessPyObject['logo']
    }



def register_business(name: str, description: str, owner, chain_local: bool, ipfs_local: bool):
    if chain_local:
        print("registering on local chain")
        client = Client()
    else:
        print("registering on remote chain")
        client = Client(node_url='wss://gesell.encointer.org', port=443)

    cid = read_cid()
    print("the cid is: ", cid);
    business_json = create_business(name, description, ipfs_local)
    f_name = f'{BUSINESSES_PATH}/{business_json["name"]}.json'
    print(f'Dumping business {business_json} to {f_name}')
    with open(f_name, 'w') as outfile:
        json.dump(business_json, outfile, indent=2)
    print("f_name, the business_dump_path: ", f_name)
    ipfs_cid = Ipfs.add_recursive(f_name, ipfs_local)
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
        image_cid = Ipfs.add_recursive(ICON_PATH, ipfs_local)
    except:
        print("failed to add image to ipfs")
    return {
        "name": name,
        "price": price,
        "community": community_identifier,
        "image_cid": image_cid
    }



def register_offering(name: str, price: int, owner, chain_local: bool, ipfs_local: bool):
    if chain_local:
        print("registering on local chain")
        client = Client()
    else:
        print("registering on remote chain")
        client = Client(node_url='wss://gesell.encointer.org')

    cid = read_cid()

    offering_json = create_offering(name, price, cid, ipfs_local)

    f_name = f'{OFFERINGS_PATH}/{offering_json["name"]}.json'
    print(f'Dumping offerings {offering_json} to {f_name}')
    with open(f_name, 'w') as outfile:
        json.dump(offering_json, outfile, indent=2)

    ipfs_cid = Ipfs.add_recursive(f_name, ipfs_local)
    print(f'Uploaded offering to ipfs: ipfs_cid: {ipfs_cid}')
    print(f"registering offering on chain for cid {cid}")
    print(client.create_offering(owner, cid, ipfs_cid))
    client.await_block()

if __name__ == '__main__':
    create_business()