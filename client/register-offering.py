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
@click.option('--bizaccount', default='', help='the account of the owner in ss58 format or raw_seed.')
@click.option('--cid', default='', help='the community identifier of the community you want to register your business in (11 digits).')
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

    biz_title = 'Select your offering json file'
    biz_file = filedialog.askopenfile(mode='r', title=biz_title)
    biz_file.name
    print(os.path.basename(biz_file.name))
    # should we handle trailling commma of last element? its in general not allowed in python
    businessPyObject = json.load(biz_file)
    biz_file.close()
    businessPyObject['price'] = price

    with tempfile.TemporaryDirectory() as tmp:
        print('created temporary directory', tmp)
        f_name = f'{os.path.basename(biz_file.name)}_offering.json'
        with open(f_name, 'w') as outfile:
            json.dump(businessPyObject, outfile, indent=2)
            offer_cid = Ipfs.add(outfile)
            print("offer_cid is:", offer_cid)

    print(businessPyObject)
    # print(type(businessPyObject))

    # if account doesn't exist yet:
    owner = client.new_account()
    print('owner is:', owner)
    client.faucet(owner)
    print(client.create_offering(owner, cid, price, businessPyObject['logo']))
    client.await_block()


    # if account already exists and is fauceted:
    # print(client.create_business(bizaccount, cid, price, businessPyObject['logo']))
    # client.await_block()


if __name__ == '__main__':
    register_offering()