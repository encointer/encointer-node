
###############################################
#  Needs the following dependencies:
# pip install pillow
# pip install qrcode

import qrcode
import click
import secrets
from base58 import b58encode

@click.command()
@click.option('--issuer', help='name of voucher issuer')
@click.option('--cid', help='name of voucher issuer', required=True)
@click.option('--network', default='nctr-k', help='network used. one of nctr-k, nctr-r, nctr-g)', required=True)
@click.option('-n', default=1, help='number of vouchers to create')
def main(issuer, cid, network, n):
    batch_token = b58encode(secrets.token_bytes(8)).decode()
    print(f"generating QR vouchers for batch_token: {batch_token}")
    with open(f'voucher-{batch_token}.secrets', 'w') as f:
        for i in range(n):
            voucher_token = b58encode(secrets.token_bytes(24)).decode()
            voucher_uri = f"//{batch_token}/{voucher_token}"
            f.write(voucher_uri)
            print(f"generating voucher {i}: {voucher_uri}")
            payload = f"encointer-voucher\nV1.0\n{voucher_uri}\n{cid}@{network}\n\n{issuer}"
            img = qrcode.make(payload)
            img.save(f"voucher-{batch_token}-{voucher_token}.png")


if __name__ == '__main__':
    main()
