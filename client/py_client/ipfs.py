import subprocess
import os
import requests
import json

from .helpers import take_only_last_cid, generate_file_list

ASSETS_PATH = './test-data/assets'
use_ipfs_gateway = True
try:
    ipfs_api_key = os.environ['IPFS_API_KEY']
    ipfs_add_url = os.environ['IPFS_ADD_URL']
except:
    print("IPFS environment not set up for using gateway")
    use_ipfs_gateway = False

class Error(Exception):
    """Base class for exceptions in this module."""
    pass

class CouldNotResolveHost(Error):
    """"Failed to connect to host. Maybe you are not connected to the internet?"""
    pass

class UnknownError(Error):
    pass

def eval_returncode(returncode):
    if returncode == 0:
        return
    if returncode == 6:
        raise CouldNotResolveHost
    raise UnknownError

class Ipfs:
    """ Minimal wrapper for the ipfs cli """
    @staticmethod
    def add_recursive(path_to_files, local = False):
        if not (use_ipfs_gateway or local):
            return "QmP2fzfikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"
        if local:
            ret = subprocess.run(["ipfs", "add", "-rw", path_to_files], stdout=subprocess.PIPE)
            return take_only_last_cid(ret)
        else:
            headers = { }
            params = ()
            if os.path.isdir(path_to_files):
                params = (
                    ('pin', 'true'),
                    ('recursive', 'true'),
                    ('wrap-with-directory', 'true'),
                )
            else:
                params = (
                    ('pin', 'true'),
                )
            files = generate_file_list(path_to_files)
            auth = ipfs_api_key.split(":")
            response = requests.post('https://ipfs.infura.io:5001/api/v0/add', headers=headers, params=params, files=files, auth=(auth[0], auth[1]))

            for line in response.text.split("\n"):
                data = json.loads(line)
                if os.path.isfile(path_to_files):
                    return data["Name"]
                if data["Name"] == "":
                    print("hash of wrapping directory: " + data["Hash"])
                    return data["Hash"]
            return 'No cid returned'


    @staticmethod
    def add_multiple_recursive(paths, local = False):
        if not (use_ipfs_gateway or local):
            return ["QmP2fzfikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"]
        return [Ipfs.add_recursive(f, local) for f in paths]


    @staticmethod
    def add(path_to_files, local=False):
        if not (use_ipfs_gateway or local):
            return "QmWgTp4fBkxyUhnMrx4UVVqQ2McTQKJzq8yq3J5tCzdtfx"
        if local:
            ret = subprocess.run(["ipfs", "add", path_to_files], check=True, stdout=subprocess.PIPE)
            return take_only_last_cid(ret)
        else:
            ret = subprocess.run(["curl", "-sS", "-X", "POST", "-F", f"file=@{path_to_files}", "-u", ipfs_api_key, ipfs_add_url], check=True, stdout=subprocess.PIPE)
        return take_only_last_cid(ret)


    @staticmethod
    def add_multiple(paths, local = False):
        if not (use_ipfs_gateway or local):
            return ["QmP2fzfikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"]
        return [Ipfs.add(f, local) for f in paths]


