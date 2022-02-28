#!/usr/bin/env python3

import json
from py_client.client import Client
from py_client.ipfs import Ipfs
from py_client.helpers import read_cid
import json
import click
import tkinter as tk
from tkinter import filedialog
from py_client.helpers import set_local_or_remote_chain
import os
import tempfile
BUSINESSES_PATH = './test-data/bazaar/'

# Before running this script, make sure, that a community is registered on the chain (for example by running bot-community.py init)

@click.command()
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('--bizaccount', required=True, help='the account of the owner in ss58 format or raw_seed.')
@click.option('--cid', required=True, help='the community identifier of the community you want to register your business in (11 digits).')
@click.option('--price', default='0', help='price of your offering.')
@click.option('-r', '--remote_chain', default=None, help='choose remote_chain: gesell.')
def register_offering(bizaccount, cid, price, client, port, remote_chain):
    """
    Register a business on chain

    :param name: path to LocalBusiness.json with all infos specified in https://github.com/encointer/pallets/blob/master/bazaar/README.md
    :return:
    """
    client = set_local_or_remote_chain(client, port, remote_chain)

    root = tk.Tk()
    root.withdraw()

    product_title = 'Select your Product.json'
    product_file = filedialog.askopenfile(mode='r', title=product_title)
    # should we handle trailling commma of last element? its in general not allowed in python and json format should not have trailing comma
    product = json.load(product_file)
    print('product is:', product)

    assert 'name' in product, "name must be defined in json file"
    assert 'description' in product, "description must be defined in json file"
    product_file.close()
    print(f'adding product {product_file.name} to ipfs')
    product_cid = Ipfs.add(product_file.name)
    offering = {
        'itemOffered': product_cid,
        'price': price
    }
    with tempfile.TemporaryDirectory() as tmp:
        print('created temporary directory', tmp)
        file_name_without_extension = os.path.splitext(os.path.basename(product_file.name))[0]
        f_name = f'{tmp}/{file_name_without_extension}Offering.json'
        with open(f_name, 'w') as outfile:
            json.dump(offering, outfile, indent=2)
        print(f'adding offering {f_name} to ipfs')
        offer_cid = Ipfs.add(f_name)

    print(f'registering product:')
    print(f'    cid:            {cid}')
    print(f'    owner:          {bizaccount}')
    print(f'    offering url:   {offer_cid}\n')

    try:
        print(client.create_offering(bizaccount, cid, product['logo']))
        client.await_block()
    except:
        print("json file doesn't have a logo entry, please save for the logo a content identifier in the json file")


if __name__ == '__main__':
    register_offering()