#!/usr/bin/env python3

from py_client.ipfs import Ipfs
import click
import tkinter as tk
from tkinter import filedialog
import os

@click.command()
@click.option('-l', '--ipfs_local', is_flag=True, help='if set, local ipfs node is used.')
def upload_image(ipfs_local):
    """
    Register a business on chain

    :param name: path to LocalBusiness.json with all infos specified in https://github.com/encointer/pallets/blob/master/bazaar/README.md
    :return:
    """

    root = tk.Tk()
    root.withdraw()

    image_title = 'Select your image'
    bizImageFile = filedialog.askopenfile(mode='r', title=image_title)

    if bizImageFile:
        logo_path = os.path.abspath(bizImageFile.name)
        try:
            image_cid = Ipfs.add(logo_path, ipfs_local)
        except:
            print("failed to add image to ipfs")


if __name__ == '__main__':
    upload_image()
