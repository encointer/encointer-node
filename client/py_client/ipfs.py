import subprocess
import re
import warnings

ICONS_PATH = '../assets/icons'

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
