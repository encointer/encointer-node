import glob
import os


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
