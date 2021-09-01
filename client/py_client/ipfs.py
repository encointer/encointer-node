import subprocess
import re
import warnings
import os

local_ipfs_key = None

if os.path.isfile("client\py_client\config.py"):
    from config import loadIPFSkey
    loadIPFSkey()
    local_ipfs_key = os.environ['LOLCAL_IPFS_API_KEY']
    print("local_ipfs_key: ", local_ipfs_key)

ICONS_PATH = '../test-data/icons'
class Ipfs:
    """ Minimal wrapper for the ipfs cli """
    @staticmethod
    def add_recursive(path_to_files):
        ret = subprocess.run(
            ["ipfs", "add", "-rw", path_to_files], stdout=subprocess.PIPE)

        # last line contains the directory cid
        last = ret.stdout.splitlines()[-1]
        p = re.compile('Qm\\w*')
        cids = p.findall(str(last))

        if cids:
            print()
            print(cids)
            return cids[0]
        else:
            warnings.warn('No cid returned. Something happened. stderr: ')
            warnings.warn(str(ret.stderr))
            return ''

    @staticmethod
    def add_recursive_multiple(paths):
        return [Ipfs.add_recursive(f) for f in paths]

    @staticmethod
    def add_recursive_remote(path_to_files, IPFS_API_KEY):
        if IPFS_API_KEY != '':
            ret = subprocess.run(
                ["curl", "-X", "POST", "-F", f"file=@{path_to_files}", IPFS_API_KEY, "https://ipfs.infura.io:5001/api/v0/add?recursive&wrap-with-directory&quiter"], stdout=subprocess.PIPE)
        elif local_ipfs_key != None:
            ret = subprocess.run(
                ["curl", "-X", "POST", "-F", f"file=@{path_to_files}", local_ipfs_key, "https://ipfs.infura.io:5001/api/v0/add"], stdout=subprocess.PIPE)
        # last line contains the directory cid
        last = ret.stdout.splitlines()[-1]
        p = re.compile('Qm\\w*')
        cids = p.findall(str(last))
        if cids:
            print()
            print(cids)
            return cids[0]
        else:
            warnings.warn('No cid returned. Something happened. stderr: ')
            warnings.warn(str(ret.stderr))
            return ''

    @staticmethod
    def add_recursive_multiple_remote(paths, IPFS_API_KEY):
        return [Ipfs.add_recursive_remote(f, IPFS_API_KEY) for f in paths]
