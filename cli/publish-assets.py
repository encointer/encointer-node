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
        asset_dir = f'{tmp}/assets'
        icon_dir = f'{asset_dir}/icons'
        icon_dir_2x = f'{icon_dir}/2.0x'
        icon_dir_3x = f'{icon_dir}/3.0x'

        os.mkdir(asset_dir)
        os.mkdir(icon_dir)
        os.mkdir(icon_dir_2x)
        os.mkdir(icon_dir_3x)

        icon1x = icon.resize((36, 36), Image.ANTIALIAS)
        icon2x = icon.resize((72, 72), Image.ANTIALIAS)
        icon3x = icon.resize((108, 108), Image.ANTIALIAS)

        icon1x.save(fp=f'{icon_dir}/community_icon.png')
        icon2x.save(fp=f'{icon_dir_2x}/community_icon.png')
        icon3x.save(fp=f'{icon_dir_3x}/community_icon.png')

        Ipfs.add_recursive(asset_dir, ipfs_local)


if __name__ == '__main__':
    main()
