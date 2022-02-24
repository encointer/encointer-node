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

BUSINESSES_PATH = './test-data/bazaar/'

# Before running this script, make sure, that a community is registered on the chain (for example by running bot-community.py init)

@click.command()
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('--cid', default='', help='the community identifier of the community your business belongs to (11 digits)')
@click.option('-r', '--remote_chain', default=None, help='choose remote_chain: gesell.')
def register_business(cid, client, port, remote_chain):
    """
    Register a business on chain

    :param name: path to LocalBusiness.json with all infos specified in https://github.com/encointer/pallets/blob/master/bazaar/README.md
    :return:
    """
    client = set_local_or_remote_chain(client, port, remote_chain)

    root = tk.Tk()
    root.withdraw()

    biz_title = 'Select your business json file'
    biz_file = filedialog.askopenfile(mode='r', title=biz_title)
    # should we handle trailling commma of last element? its in general not allowed in python
    businessPyObject = json.load(biz_file)
    biz_file.close()

    print(businessPyObject)
    # print(type(businessPyObject))

    owner = client.new_account()
    print('owner is:', owner)
    client.faucet(owner)
    print(client.create_business(owner, cid, businessPyObject['logo']))
    client.await_block()


if __name__ == '__main__':
    register_business()