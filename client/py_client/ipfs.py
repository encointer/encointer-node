import subprocess
import warnings
import os
from .helpers import take_only_last_cid

ICONS_PATH = '../test-data/icons'
ipfs_api_key = os.environ['IPFS_API_KEY']
ipfs_add_url = os.environ['IPFS_ADD_URL']
class Ipfs:
    """ Minimal wrapper for the ipfs cli """
    @staticmethod
    def add(path_to_files, local = False):
        ret = ''
        if local:
            ret = subprocess.run(["ipfs", "add", path_to_files], stdout=subprocess.PIPE) 
        else: 
            ret = subprocess.run(["curl", "-X", "POST", "-F", f"file=@{path_to_files}", "-u", ipfs_api_key, ipfs_add_url], stdout=subprocess.PIPE)
        return take_only_last_cid(ret)

    @staticmethod
    def add_multiple(paths, local = False):
        return [Ipfs.add(f, local) for f in paths]