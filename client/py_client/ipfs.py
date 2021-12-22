import subprocess
import warnings
import os
import requests
import json

from .helpers import take_only_last_cid
from pathlib import Path
ICONS_PATH = './test-data/icons'
use_ipfs_gateway = True
try:
    ipfs_api_key = os.environ['IPFS_API_KEY']
    ipfs_add_url = os.environ['IPFS_ADD_URL']
except:
    print("IPFS environment not set up for using gateway")
    use_ipfs_gateway = False

class Ipfs:
    """ Minimal wrapper for the ipfs cli """
    @staticmethod
    def add(path_to_files, local = False):
        if not (use_ipfs_gateway or local):
            return "QmP2fzfikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"
        if local:
            ret = subprocess.run(["ipfs", "add", "-rw", path_to_files], stdout=subprocess.PIPE)
            print(ret)
            return take_only_last_cid(ret)
        else:
            headers = { }
            params = (
                ('pin', 'true'),
                ('recursive', 'true'),
                ('wrap-with-directory', 'true'),
            )
            files = Ipfs.generate_file_list(path_to_files)
            auth = ipfs_api_key.split(":")

            response = requests.post('https://ipfs.infura.io:5001/api/v0/add', headers=headers, params=params, files=files, auth=(auth[0], auth[1]))

            for line in response.text.split("\n"):
                print(line)
                data = json.loads(line)
                if data["Name"] == "":
                    print("final hash: " + data["Hash"])
                    return data["Hash"]
            return ""


    @staticmethod
    def add_multiple(paths, local = False):
        if not (use_ipfs_gateway or local):
            return ["QmP2fzfikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"]
        return [Ipfs.add(f, local) for f in paths]


    @staticmethod
    def generate_file_list(path_to_files):
        args = []
        for dir_, _, files in os.walk(path_to_files):
            for file_name in files:
                rel_path = os.path.relpath(os.path.join(dir_, file_name), str(Path(path_to_files).parent))
                rel_path = Path(rel_path)
                with open(os.path.join(dir_, file_name), 'rb') as file:
                    args += [(rel_path.as_posix(), file.read())]
        return args



