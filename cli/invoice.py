#!/usr/bin/env python3
###############################################
#  Needs the following dependencies:
# pip install pillow
# pip install qrcode

import qrcode
import click
import secrets
from base58 import b58encode

@click.command()
@click.option('--recipient', help='name of recipient', required=True)
@click.option('--account', help='account of recipient with ss58 prefix 42', required=True)
@click.option('--amount', default="", help='invoice amount')
@click.option('--cid', default="u0qj92QX9PQ", help='community identifier')
@click.option('--network', default='nctr-k', help='network used. one of nctr-k, nctr-r, nctr-g)')
def main(recipient, account, amount, cid, network):
    print(f"generating invoice QR for {account}")
    payload = f"encointer-invoice\nV1.0\n{account}\n{cid}\n{amount}\n{recipient}"
    img = qrcode.make(payload)
    img.save(f"invoice-{recipient}-{account}-{amount}-{cid}.png")


if __name__ == '__main__':
    main()
