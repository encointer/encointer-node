import subprocess
import warnings
import os
from .helpers import take_only_last_cid

ICONS_PATH = '../test-data/icons'

class Ipfs:
    """ Minimal wrapper for the ipfs cli """
    @staticmethod
    def add_recursive(path_to_files):
        ret = subprocess.run(["ipfs", "add", "-rw", path_to_files], stdout=subprocess.PIPE)
        return take_only_last_cid(ret)

    @staticmethod
    def add_recursive_multiple(paths):
        return [Ipfs.add_recursive(f) for f in paths]

    # doesn't work yet, for the remote folder adding with infura, only files
    @staticmethod
    def add_recursive_remote(path_to_files, ipfs_api_key, ipfs_add_url):
        if ipfs_api_key:
            ret = subprocess.run(["curl", "-X", "POST", "-F", f"file=@{path_to_files}", ipfs_api_key, ipfs_add_url], stdout=subprocess.PIPE)
            return take_only_last_cid(ret)
        else:
            warnings.warn('No IPFS_API_KEY entered. Please add using the --ipfs-api-key option. stderr: ')
            # warnings.warn(str(ret.stderr))
            return ''

    @staticmethod
    def add_recursive_multiple_remote(paths, ipfs_api_key, ipfs_add_url):
        return [Ipfs.add_recursive_remote(f, ipfs_api_key, ipfs_add_url) for f in paths]
