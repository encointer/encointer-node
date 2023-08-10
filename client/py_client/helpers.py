import glob
import os
import subprocess
import re
from os import path
from pathlib import Path
from .client import Client
import warnings

def purge_prompt(path: str, file_description: str):
    files = glob.glob(path + '/*')
    if files:
        print(f'{path} already contains {len(files)} {file_description}.')
        should_clear = input(f'Do you want to purge the {path}? [y, n]')
        if should_clear == 'y':
            [os.remove(f) for f in files]
            print(f'Purged the {path}.')
        else:
            print(f'Leaving {path} as is.')


def write_cid(cid: str):
    f = open('cid.txt', 'w')
    f.write(cid)
    f.close()


def read_cid():
    f = open('cid.txt', 'r')
    cid = f.read()
    f.close()
    return cid


def mkdir_p(path):
    """ Surprisingly, there is no simple function in python to create a dir if it does not exist."""
    return subprocess.run(['mkdir', '-p', path])


# this method takes the last content identifier, which is the one of the whole folder, for a file, there is only one cid so it works, too.
def take_only_last_cid(ret_cids):
        # last line contains the directory cid
        last = ret_cids.stdout.splitlines()[-1]
        p = re.compile('Qm\\w*')
        cids = p.findall(str(last))

        if cids:
            print(cids[0])
            return cids[0]
        else:
            warnings.warn('No cid returned. Something happened. stderr: ')
            warnings.warn(str(ret_cids.stderr))
            return ''


def generate_file_list(path_to_files):
    args = []
    if os.path.isdir(path_to_files):
        for dir_, _, files in os.walk(path_to_files):
            for file_name in files:
                rel_path = os.path.relpath(os.path.join(dir_, file_name), str(Path(path_to_files).parent))
                rel_path = Path(rel_path)
                with open(os.path.join(dir_, file_name), 'rb') as file:
                    args += [(rel_path.as_posix(), file.read())]
    else:
        rel_path = ''
        rel_path = Path(rel_path)
        with open(os.path.abspath(path_to_files), 'rb') as file:
            args += [(rel_path.as_posix(), file.read())]
    return args


def set_local_or_remote_chain(client: str, port: str, node_url: str):
    if node_url is None:
        client = Client(rust_client=client, port=port)
    else:
        if node_url == "gesell":
            client = Client(rust_client=client, node_url='wss://gesell.encointer.org', port=443)
        elif node_url == "rococo":
            client = Client(rust_client=client, node_url='wss://rococo.api.encointer.org', port=443)
        elif node_url == "kusama":
            client = Client(rust_client=client, node_url='wss://kusama.api.encointer.org', port=443)
        else:
            raise Exception("You need to choose a valid remote chain")
    return client
