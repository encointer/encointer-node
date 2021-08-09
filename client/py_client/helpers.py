import glob
import os


def purge_prompt(path: str, file_description: str):
    files = glob.glob(path + '/*')
    if files:
        print(f'Keystore already contains {len(files)} {file_description}.')
        should_clear = input(f'Do you want to purge the {path}? [y, n]')
        if should_clear == 'y':
            [os.remove(f) for f in files]
            print(f'Purged the {path}.')
        else:
            print(f'Leaving {path} as is.')
