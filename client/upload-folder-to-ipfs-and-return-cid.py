#!/usr/bin/env python3

from py_client.ipfs import Ipfs
import click
import tkinter as tk
from tkinter import filedialog
import os

@click.command()
@click.option('-l', '--ipfs_local', is_flag=True, help='if set, local ipfs node is used.')
def upload_folder(ipfs_local):
    """
    Upload a folder to ipfs
    cid of the folder
    """
    root = tk.Tk()
    root.withdraw()

    directory_title = 'Select your folder'
    directory = filedialog.askdirectory(title=directory_title)

    if directory:
        folder_path = os.path.abspath(directory)
        try:
            Ipfs.add_recursive(folder_path, ipfs_local)
        except Exception as ex:
            print("failed to add folder to ipfs")
            template = "An exception of type {0} occurred. Arguments:\n{1!r}"
            message = template.format(type(ex).__name__, ex.args)
            print(message)

if __name__ == '__main__':
    upload_folder()
