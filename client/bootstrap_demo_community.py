#!python
import argparse
import subprocess
import re
import warnings
# import click

from client.client import Client
from client.scheduler import CeremonyPhase

cli = ["../target/release/encointer-client.py-notee -p 9444"]

account1 = '//Alice'
account2 = '//Bob'
account3 = '//Charlie'
accounts = [account1, account2, account3]


def upload(path_to_files):
    ret = subprocess.run("ipfs add -rw " + path_to_files, stdout=subprocess.PIPE)

    # last line contains the directory cid
    last = ret.stdout.splitlines()[-1]
    p = re.compile('Qm\\w*')
    cids = p.findall(str(last))

    if cids:
        print()
        print(cids)
        return cids[0]
    else:
        warnings.warn('No cid returned something happened. stderr: ')
        warnings.warn(str(ret.stderr))
        return ''


def main(client=Client()):
    # cid = client.new_community('test-locations-mediterranean.json')
    cid = '41eSfKJrhrR6CYxPfUbwAN18R77WbxXoViRWQMAF4hJB'
    print(cid)

    print(client.list_communities())

    client.go_to_phase(CeremonyPhase.REGISTERING)

    # charlie has no genesis funds
    print('Dripping faucets to Charlie...')
    client.faucet([account3])

    print('Registering Participants...')
    [client.register_participant(b, cid) for b in accounts]

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait}, such that xt's are processed")
    client.await_block(blocks_to_wait)

    print('List participants')
    print(client.list_participants(cid))

    print('Going to next phase ')
    client.next_phase()

    print('Listing meetups')
    print(client.list_meetups(cid))

    print('Going to next phase ')
    client.next_phase()

    print('Starting meetup')
    print('Creating claims')
    vote = len(accounts)
    claim1 = client.new_claim(account1, vote, cid)
    claim2 = client.new_claim(account2, vote, cid)
    claim3 = client.new_claim(account3, vote, cid)

    print('Signing claims')
    witness1_2 = client.sign_claim(account1, claim2)
    witness1_3 = client.sign_claim(account1, claim3)

    witness2_1 = client.sign_claim(account2, claim1)
    witness2_3 = client.sign_claim(account2, claim3)

    witness3_1 = client.sign_claim(account3, claim1)
    witness3_2 = client.sign_claim(account3, claim2)

    print('sending witnesses to chain')
    client.register_attestations(account1, [witness2_1, witness3_1])
    client.register_attestations(account2, [witness1_2, witness3_2])
    client.register_attestations(account3, [witness1_3, witness2_3])

    print(f"Waiting for {blocks_to_wait}, such that xt's are processed")
    client.await_block(blocks_to_wait)

    print('Listing Attestations')
    print(client.list_attestations(cid))

    print('Going to next phase ')
    client.next_phase()

    print(f'Balances for new community with cid: {cid}')
    bal = [client.balance(a, cid=cid) for a in accounts]
    abs = list(zip(accounts, bal))

    [print(f'Account balance for {ab[0]} is {ab[1]}') for ab in abs]


if __name__ == '__main__':
    main()
