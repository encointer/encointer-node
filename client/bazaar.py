#!/usr/bin/env python3
"""
A helper script to manage businesses and offerings for the bazaar

register a business with an offering
1. `./bootstrap-demo-community.py`
2. `./bazaar.py --cid sqm1v79dF6b --bizaccount //Alice register-business ./test-data/bazaar/EdisonPaula/businesses/weidlibraeu/Weidlibraeu.json`
3. select test-data/bazaar/EdisonPaula/businesses/weidlibraeu/Weidlibraeu.json
4. should create business and upload json to ipfs with url/(ipfs cid) QmWYjfdsScf2mBxKgGbBq822FzRNkB5nG6wjK2NKLw1Ewz
5. `./bazaar.py --cid sqm1v79dF6b --bizaccount //Alice register-offering ./test-data/bazaar/EdisonPaula/businesses/weidlibraeu/products/WeidlibraeuBier.json `
6. select test-data/bazaar/EdisonPaula/businesses/weidlibraeu/products/WeidlibraeuBier.json
7. should create offering and upload json to ipfs with url QmQsANG7NktUntHyVPa1EHskySGs21AkZ8pTUEDvEUbQcz

list businesses and offerings
./bazaar.py --cid sqm1v79dF6b list-businesses
./bazaar.py --cid sqm1v79dF6b [--bizaccount //Alice] list-offerings

the above setup assumes you have an infura api set up:
```
export IPFS_ADD_URL=https://ipfs.infura.io:5001/api/v0/add
export IPFS_API_KEY=key:secret
```
"""

from py_client.ipfs import Ipfs
import json
import click
from py_client.helpers import set_local_or_remote_chain
import tempfile
import os




@click.group()
@click.option('--cid', required=True, help='the community identifier of the community you want to register your business in (11 digits).')
@click.option('--bizaccount', required=False, help='the account of the owner in ss58 format or raw_seed.')
@click.option('--price', default='0', help='price of your offering.')
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('-r', '--remote_chain', default=None, help='choose remote chain: gesell.')
@click.pass_context
def cli(ctx, client, port, cid, bizaccount, price, remote_chain):
    ctx.ensure_object(dict)
    cl = set_local_or_remote_chain(client, port, remote_chain)
    ctx.obj['client'] = cl
    ctx.obj['port'] = port
    ctx.obj['cid'] = cid
    ctx.obj['bizaccount'] = bizaccount
    # ctx.obj['ipfs_local'] = ipfs_local
    ctx.obj['remote_chain'] = remote_chain
    ctx.obj['price'] = price


@cli.command()
@click.argument('specfile', type=click.File('r'))
@click.pass_obj
def register_business(ctx, specfile):
    """
    Register a business on chain and upload metadata to ipfs.
    :param cid: on chain registered community identifier\n
    :param bizaccount: on chain registered business account in ss58 or raw_seed format\n
    :param specfile: json file describing the business which should be according to the folowing scheme: https://github.com/encointer/pallets/blob/master/bazaar/README.md
    :return:
    """
    client = ctx['client']

    business = json.load(specfile)
    print('business is:', business)

    assert 'name' in business, "name must be defined in json file"
    assert 'description' in business, "description must be defined in json file"

    print(f'adding business {specfile.name} to ipfs')
    business_cid = Ipfs.add(specfile.name)

    print(f'registering business:')
    print(f'    cid:            {ctx["cid"]}')
    print(f'    owner:          {ctx["bizaccount"]}')
    print(f'    business url:   {business_cid}\n')

    try:
        print(client.create_business(ctx['bizaccount'], ctx['cid'], business_cid))
        client.await_block()
    except:
        print("error creating a business entry")


@cli.command()
@click.argument('specfile', type=click.File('r'))
@click.pass_obj
def register_offering(ctx, specfile):
    """
    Register a product on chain and upload metadata to ipfs.\n
     :param cid: on chain registered community identifier\n
    :param bizaccount: on chain registered business account in ss58 or raw_seed format\n
    :param price: price for the product you want to offer\n
    :param specfile: json file describing the product which should be according to the folowing scheme: https://github.com/encointer/pallets/blob/master/bazaar/README.md
    :return:
    """
    client = ctx['client']

    product = json.load(specfile)
    print('product is:', product)

    assert 'name' in product, "name must be defined in json file"
    assert 'description' in product, "description must be defined in json file"

    print(f'adding product {specfile.name} to ipfs')
    product_cid = Ipfs.add(specfile.name)
    offering = {
        'itemOffered': product_cid,
        'price': ctx['price']
    }
    with tempfile.TemporaryDirectory() as tmp:
        print('created temporary directory', tmp)
        file_name_without_extension = os.path.splitext(os.path.basename(specfile.name))[0]
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
        print(client.create_offering(ctx['bizaccount'], ctx['cid'], offer_cid))
        client.await_block()
    except:
        print("error creating an offering entry")

@cli.command()
@click.pass_obj
def list_businesses(ctx):
    """
    List all offerings registered for a given cid.\n
    :param cid: on chain registered community identifier\n
    :return:
    """
    client = ctx['client']
    print(client.list_businesses(ctx['cid']))

@cli.command()
@click.pass_obj
def list_offerings(ctx):
    """
    List all offerings registered for a given cid.\n
    :param cid: on chain registered community identifier\n
    :param bizaccount: on chain registered business account in ss58 or raw_seed format\n
    :return:
    """
    client = ctx['client']
    print(ctx["bizaccount"])
    if ctx["bizaccount"] is None:
        print(client.list_offerings(ctx['cid']))
    else:
        print(client.list_offerings_for_business(ctx['cid'], ctx["bizaccount"]))

if __name__ == '__main__':
    cli(obj={})
