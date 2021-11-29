import glob
import os
import subprocess
import re
import shutil
from os import path

from .client import Client

def zip_folder(name: str, folder_abs_path: str):
    return shutil.make_archive(f"{name}","zip", folder_abs_path)

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
            print()
            print(cids)
            return cids[0]
        else:
            warnings.warn('No cid returned. Something happened. stderr: ')
            warnings.warn(str(ret_cids.stderr))
            return ''


def set_local_or_remote_chain(client: str, port: str, node_url: str):
    if node_url is None:
        client = Client(rust_client=client, port=port)
    else:
        client = Client(rust_client=client, node_url='wss://gesell.encointer.org', port=443)
    return client
