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
    Upload an image to ipfs
    :return: cid of the image
    """
    root = tk.Tk()
    root.withdraw()

    image_title = 'Select your image'
    bizImageFile = filedialog.askopenfile(mode='r', title=image_title)

    if bizImageFile:
        logo_path = os.path.abspath(bizImageFile.name)
        try:
            Ipfs.add(logo_path, ipfs_local)
        except Exception as ex:
            print("failed to add image to ipfs")
            template = "An exception of type {0} occurred. Arguments:\n{1!r}"
            message = template.format(type(ex).__name__, ex.args)
            print(message)


if __name__ == '__main__':
    upload_image()
