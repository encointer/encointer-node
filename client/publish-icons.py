#!/usr/bin/env python3
#
# helper to publish a community icon to ipfs in all required resolutions to be used by the app

import click
import os
from py_client.ipfs import Ipfs
import tempfile
from PIL import Image

MIN_PIX = 108


@click.command()
@click.argument('icon', type=click.File('rb'))
@click.option('-l', '--ipfs_local', is_flag=True, help='if set, local ipfs node is used.')
def main(ipfs_local, icon):

    icon = Image.open(icon)
    w, h = icon.size
    if min(w, h) < MIN_PIX:
        raise NameError('image too small')

    with tempfile.TemporaryDirectory() as tmp:
        print('created temporary directory', tmp)
        os.mkdir(tmp + "/icons/")
        os.mkdir(tmp + "/icons/2.0x")
        os.mkdir(tmp + "/icons/3.0x")
        icon1x = icon.resize((36, 36), Image.ANTIALIAS)
        icon2x = icon.resize((72, 72), Image.ANTIALIAS)
        icon3x = icon.resize((108, 108), Image.ANTIALIAS)

        icon1x.save(fp=tmp+"/icons/community_icon.png")
        icon2x.save(fp=tmp+"/icons/2.0x/community_icon.png")
        icon3x.save(fp=tmp+"/icons/3.0x/community_icon.png")

        Ipfs.add_recursive(tmp+"/icons", ipfs_local)


if __name__ == '__main__':
    main()
