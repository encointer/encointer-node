#!/usr/bin/env python3

from py_client.ipfs import Ipfs
import json
import click
import tkinter as tk
from tkinter import filedialog
from py_client.helpers import set_local_or_remote_chain
import tempfile
import os

# Before running this script, make sure, that a community is registered on the chain (for example by running bot-community.py init)


@click.group()
@click.option('--cid', required=True, help='the community identifier of the community you want to register your business in (11 digits).')
@click.option('--bizaccount', required=True, help='the account of the owner in ss58 format or raw_seed.')
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('--price', default='0', help='price of your offering.')
@click.option('-r', '--remote_chain', default=None, help='choose remote chain: gesell.')
@click.pass_context
def cli(ctx, client, port, cid, bizaccount, price, remote_chain):
    ctx.ensure_object(dict)
    cl = set_local_or_remote_chain(client, port, remote_chain)
    root = tk.Tk()
    root.withdraw()
    ctx.obj['client'] = cl
    ctx.obj['port'] = port
    ctx.obj['cid'] = cid
    ctx.obj['bizaccount'] = bizaccount
    # ctx.obj['ipfs_local'] = ipfs_local
    ctx.obj['remote_chain'] = remote_chain
    ctx.obj['price'] = price


@cli.command()
@click.pass_obj
def register_business(ctx):
    """
    Register a business on chain and upload to ipfs.
    Select business.json which should be according to the folowing scheme:\n
    https://github.com/encointer/pallets/blob/master/bazaar/README.md\n
    :param cid: on chain registered community identifier\n
    :param bizaccount: on chain registered business account in ss58 or raw_seed format\n
    :return:
    """
    client = ctx['client']

    biz_title = 'Select your business json file'
    biz_file = filedialog.askopenfile(mode='r', title=biz_title)
    # should we handle trailling commma of last element? its in general not allowed in python
    business = json.load(biz_file)
    biz_file.close()
    print('business is:', business)

    assert 'name' in business, "name must be defined in json file"
    assert 'description' in business, "description must be defined in json file"

    print(f'adding business {biz_file.name} to ipfs')
    business_cid = Ipfs.add(biz_file.name)

    print(f'registering business:')
    print(f'    cid:            {ctx["cid"]}')
    print(f'    owner:          {ctx["bizaccount"]}')
    print(f'    business url:   {business_cid}\n')

    try:
        print(client.create_business(ctx['bizaccount'], ctx['cid'], business['logo']))
        client.await_block()
    except:
        print("json file doesn't have a logo entry, please save for the logo a content identifier in the json file")


@cli.command()
@click.pass_obj
def register_offering(ctx):
    """
    Register a product on chain and upload to ipfs.\n
    Select Product.json which should be according to the folowing scheme:\n
    https://github.com/encointer/pallets/blob/master/bazaar/README.md\n
    :param cid: on chain registered community identifier\n
    :param bizaccount: on chain registered business account in ss58 or raw_seed format\n
    :param price: price for the product you want to offer\n
    :return:
    """
    client = ctx['client']

    product_title = 'Select your Product.json'
    product_file = filedialog.askopenfile(mode='r', title=product_title)
    # should we handle trailling commma of last element? its in general not allowed in python and json format should not have trailing comma
    product = json.load(product_file)
    product_file.close()
    print('product is:', product)

    assert 'name' in product, "name must be defined in json file"
    assert 'description' in product, "description must be defined in json file"

    print(f'adding product {product_file.name} to ipfs')
    product_cid = Ipfs.add(product_file.name)
    offering = {
        'itemOffered': product_cid,
        'price': ctx['price']
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
    print(f'    cid:            {ctx["cid"]}')
    print(f'    owner:          {ctx["bizaccount"]}')
    print(f'    offering url:   {offer_cid}\n')

    try:
        print(client.create_offering(ctx['bizaccount'], ctx['cid'], product['logo']))
        client.await_block()
    except:
        print("json file doesn't have a logo entry, please save for the logo a content identifier in the json file")


if __name__ == '__main__':
    cli(obj={})
