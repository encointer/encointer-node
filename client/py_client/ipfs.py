import subprocess
import warnings
import os
from .helpers import take_only_last_cid

ICONS_PATH = '../test-data/icons'

class Ipfs:
    """ Minimal wrapper for the ipfs cli """
    @staticmethod
    def add(path_to_files):
        ret = subprocess.run(["ipfs", "add", path_to_files], stdout=subprocess.PIPE)
        return take_only_last_cid(ret)

    @staticmethod
    def add_multiple(paths):
        return [Ipfs.add(f) for f in paths]

    @staticmethod
    def add_remote(path_to_files, ipfs_api_key, ipfs_add_url):
        ret = subprocess.run(["curl", "-X", "POST", "-F", f"file=@{path_to_files}", "-u", ipfs_api_key, ipfs_add_url], stdout=subprocess.PIPE)
        return take_only_last_cid(ret)

    @staticmethod
    def add_multiple_remote(paths, ipfs_api_key, ipfs_add_url):
        return [Ipfs.add_remote(f, ipfs_api_key, ipfs_add_url) for f in paths]
    