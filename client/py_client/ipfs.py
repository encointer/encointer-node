import subprocess
import re
import warnings
import os

ICONS_PATH = '../test-data/icons'


class Ipfs:
    """ Minimal wrapper for the ipfs cli """
    @staticmethod
    def add_recursive(path_to_files):
        ret = subprocess.run(["ipfs", "add", "-rw", path_to_files], stdout=subprocess.PIPE)
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
        # Ipfs.refactoredCID(ret)

    @staticmethod
    def refactoredCID(ret_cid):
        # last line contains the directory cid
        last = ret_cid.stdout.splitlines()[-1]
        p = re.compile('Qm\\w*')
        cids = p.findall(str(last))

        if cids:
            print()
            print(cids)
            return cids[0]
        else:
            warnings.warn('No cid returned. Something happened. stderr: ')
            warnings.warn(str(ret_cid.stderr))
            return ''

    @staticmethod
    def add_recursive_multiple(paths):
        return [Ipfs.add_recursive(f) for f in paths]

    @staticmethod
    def add_recursive_remote(path_to_files, IPFS_API_KEY):
        if IPFS_API_KEY != '':
            ret = subprocess.run(["curl", "-X", "POST", "-F", f"file=@{path_to_files}", IPFS_API_KEY,
                                 "https://ipfs.infura.io:5001/api/v0/add?recursive&wrap-with-directory&quiter"], stdout=subprocess.PIPE)
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
            # Ipfs.refactoredCID(ret)
        else:
            warnings.warn('No IPFS_API_KEY entered. Please add using the --ipfs-api-key option. stderr: ')
            # warnings.warn(str(ret.stderr))
            return ''

    @staticmethod
    def add_recursive_multiple_remote(paths, IPFS_API_KEY):
        return [Ipfs.add_recursive_remote(f, IPFS_API_KEY) for f in paths]
