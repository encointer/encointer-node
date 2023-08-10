#!/usr/bin/env python3
"""
Demonstrate the bootstrapping of an Encointer community on a *dev* chain.

start node with
  ../target/release/encointer-node-notee --dev --tmp --ws-port 9945 --enable-offchain-indexing true --rpc-methods unsafe

or start parachain with  
then run this script
  ./bootstrap_demo_community.py --port 9945

"""

import json
import os
import click

from py_client.client import Client
from py_client.scheduler import CeremonyPhase
from py_client.ipfs import Ipfs, ASSETS_PATH

account1 = '//Alice'
account2 = '//Bob'
account3 = '//Charlie'
accounts = [account1, account2, account3]

TEST_DATA_DIR = './test-data/'
TEST_LOCATIONS_MEDITERRANEAN = 'test-locations-mediterranean.json'


def check_participant_count(client, cid, type, number):
    participants_list = client.list_participants(cid)
    print(participants_list)
    expected_string = f"""Querying {type}Registry
number of participants assigned:  {number}"""
    if not expected_string in participants_list:
        print(f"ERROR: Not {number} {type}s registered")
        exit(1)

def check_reputation(client, cid, account, cindex, reputation):
    rep = client.reputation(account)
    print(rep)
    if (str(cindex), f" {cid}", reputation) not in rep:
        print(f"Reputation for {account} in cid {cid} cindex {cindex} is not {reputation}")
        exit(1)


def perform_meetup(client, cid, accounts):
    print('Starting meetup...')

    print('Attest other attendees...')
    for account in accounts:
        client.attest_attendees(account, cid, [a for a in accounts if a != account])


def update_spec_with_cid(file, cid):
    with open(file, 'r+') as spec_json:
        spec = json.load(spec_json)
        spec['community']['meta']['assets'] = cid
        print(spec)
        # go to beginning of the file to overwrite
        spec_json.seek(0)
        json.dump(spec, spec_json, indent=2)
        spec_json.truncate()


def create_community(client, spec_file_path, ipfs_local):
    # non sudoer can create community
    cid = client.new_community(spec_file_path, signer=account2)
    if len(cid) > 10:
        print(f'Registered community with cid: {cid}')
    else:
        exit(1)

    print('Uploading assets to ipfs')
    root_dir = os.path.realpath(ASSETS_PATH)
    ipfs_cid = Ipfs.add_recursive(root_dir, ipfs_local)

    print(f'Updating Community spec with ipfs cid: {ipfs_cid}')
    update_spec_with_cid(spec_file_path, ipfs_cid)

    return cid


def register_participants_and_perform_meetup(client, cid, accounts):
    print(client.list_communities())
    client.go_to_phase(CeremonyPhase.Registering)

    print(f'Registering Participants for cid: {cid}')
    [client.register_participant(b, cid) for b in accounts]

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print(client.list_participants(cid))
    client.next_phase()

    print('Listing meetups')
    print(client.list_meetups(cid))
    client.next_phase()

    print(f'Performing meetups for cid {cid}')
    perform_meetup(client, cid, accounts)

    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print(client.list_attestees(cid))


def faucet(client, cid, accounts):
    # charlie has no genesis funds
    print('Faucet is dripping...')
    client.faucet(accounts, is_faucet=True)

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)


def fee_payment_transfers(client, cid):
    print(f'Transferring 0.5CC from //Alice to //Eve')
    client.transfer(cid, '//Alice', '//Eve', '0.5', pay_fees_in_cc=False)

    print(f'Transferring all CC from //Eve to //Ferdie')
    client.transfer_all(cid, '//Eve', '//Ferdie', pay_fees_in_cc=True)
    if client.balance('//Eve', cid=cid) > 0 or client.balance('//Ferdie', cid=cid) == 0:
        print("transfer_all failed")
        exit(1)


def claim_rewards(client, cid, account, meetup_index=None, all=False, pay_fees_in_cc=False):
    print("Claiming rewards")
    client.claim_reward(account, cid, meetup_index=meetup_index, all=all, pay_fees_in_cc=pay_fees_in_cc)
    client.await_block(3)


def test_reputation_caching(client, cid, account):
    register_participants_and_perform_meetup(client, cid, accounts)
    client.next_phase()
    client.await_block(1)
    # query reputation to set the cache in the same phase as claiming rewards
    # so we would have a valid cache value, but the cache should be invalidated
    # anyways because of the dirty bit
    client.reputation(account1)
    claim_rewards(client, cid, account1)

    # check if the reputation cache was updated
    rep = client.reputation(account1)
    print(rep)
    if ('1', ' sqm1v79dF6b', 'VerifiedLinked') not in rep or ('2', ' sqm1v79dF6b', 'VerifiedLinked') not in rep or ('3', ' sqm1v79dF6b', 'VerifiedUnlinked') not in rep:
        print("wrong reputation")
        exit(1)

    # test if reputation cache is invalidated after registration
    print(f'Registering Participants for Cid: {cid}')
    [client.register_participant(b, cid) for b in accounts]

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    rep = client.reputation(account1)
    print(rep)
    # after the registration the second reputation should now be linked
    if ('3', ' sqm1v79dF6b', 'VerifiedLinked') not in rep:
        print("reputation not linked")
        exit(1)

    client.next_phase()
    client.next_phase()
    client.next_phase()
    client.await_block(1)

    # check if reputation cache gets updated after phase change
    print(client.purge_community_ceremony(cid, 1, 5))
    client.await_block(1)

    client.next_phase()
    rep = client.reputation(account1)
    # after phase change cache will be updated
    if not len(rep) == 0:
        print("reputation was not cleared")
        exit(1)

    client.next_phase()
    client.next_phase()
    client.await_block(1)
    # registering


def test_unregister_and_upgrade_registration(client, cid):
    newbie = client.create_accounts(1)[0]
    faucet(client, cid, [newbie])

    register_participants_and_perform_meetup(client, cid, accounts + [newbie])
    client.next_phase()
    client.await_block(1)

    client.register_participant(newbie, cid)
    client.await_block(1)
    print(client.list_participants(cid))
    check_participant_count(client, cid, "Newbie", 1)

    claim_rewards(client, cid, account1, pay_fees_in_cc=True)
    client.await_block(1)

    check_reputation(client, cid, newbie, 6, "VerifiedUnlinked")
    client.upgrade_registration(newbie, cid)
    client.await_block(1)

    check_participant_count(client, cid, "Newbie", 0)
    check_participant_count(client, cid, "Reputable", 1)

    check_reputation(client, cid, newbie, 6, "VerifiedLinked")

    client.unregister_participant(newbie, cid, cindex=6)
    client.await_block(3)
    check_participant_count(client, cid, "Reputable", 0)

    check_reputation(client, cid, newbie, 6, "VerifiedUnlinked")


def test_endorsements_by_reputables(client, cid):
    newbies = client.create_accounts(7)
    faucet(client, cid, newbies)

    register_participants_and_perform_meetup(client, cid, accounts + newbies[:1])
    client.next_phase()
    client.await_block(1)
    claim_rewards(client, cid, account1, pay_fees_in_cc=True)
    client.await_block(1)
    # newbies[0] is now reputable
    check_participant_count(client, cid, "Endorsee", 0)

    # endorsement works before registration
    client.endorse_newcomers(cid, newbies[0], [newbies[1]])
    client.await_block(1)
    client.register_participant(newbies[1], cid)
    client.await_block(1)
    check_participant_count(client, cid, "Endorsee", 1)

    # endorsement works after registration
    for i in range(2, 6):
        client.register_participant(newbies[i], cid)
        client.await_block(1)
        client.endorse_newcomers(cid, newbies[0], [newbies[i]])
        client.await_block(1)

        check_participant_count(client, cid, "Endorsee", i)

    # all tickets used, should fail
    print(client.endorse_newcomers(cid, newbies[0], [newbies[6]]))
    client.await_block(2)
    # endorsee count is still 5
    check_participant_count(client, cid, "Endorsee", 5)

def balance(x):
    return x * 10**12


def test_faucet(client, cid):
    client.set_faucet_reserve_amount("//Alice", balance(3000))
    client.await_block(2)
    balance_bob = client.balance("//Bob")
    client.create_faucet("//Bob", "TestFaucet", balance(10000), balance(1000), [cid], cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    faucet_account = "5CRaq3MpDT1j1d7xoaG3LDwqgC5AoTzRtGptSHm2yFrWoVid"
    print(client.balance("//Bob"), flush=True)
    print(balance_bob, flush=True)
    faucet_account_balance = client.balance(faucet_account)
    print(f"faucet_account_balance: {faucet_account_balance}", flush=True)
    expected_facuet_balance = balance(10000)
    if(not faucet_account_balance == expected_facuet_balance):
        print(f"Wrong Faucet balance after faucet creation: expected: {expected_facuet_balance}, actual: {faucet_account_balance}")
        exit(1)
    expected_bob_balance = balance(13000)
    balance_bob_new = balance_bob - client.balance("//Bob")
    if(not balance_bob_new == expected_bob_balance):
        print(f"Wrong Bob balance after faucet creation: expected: {expected_bob_balance}, actual: {balance_bob_new}")
        exit(1)
    print('Faucet created', flush=True)

    balance_charlie = client.balance("//Charlie")
    client.drip_faucet("//Charlie", faucet_account, 1, cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    if(not client.balance("//Charlie") == balance_charlie + balance(1000)):
        print(f"Drip failed")
        exit(1)
    print('Faucet dripped', flush=True)

    balance_bob = client.balance("//Bob")
    client.dissolve_faucet("//Alice", faucet_account, "//Eve")
    client.await_block(2)

    if(not client.balance("//Eve") == balance(9000)):
        print(f"Dissolve failed")
        exit(1)
    
    if(not client.balance("//Bob") == balance_bob + balance(3000)):
        print(f"Dissolve failed")
        exit(1)
    
    print('Faucet dissolved', flush=True)
    client.create_faucet("//Bob", "TestFaucet", balance(10000), balance(9000), [cid], cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    if(not client.balance(faucet_account) == balance(10000)):
        print(f"Faucet creation failed")
        exit(1)
    print('Faucet created', flush=True)
    client.drip_faucet("//Charlie", faucet_account, 1, cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    print('Faucet dripped', flush=True)
    balance_bob = client.balance("//Bob")
    client.close_faucet("//Bob", faucet_account, cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    if(not client.balance(faucet_account) == 0):
        print(f"Faucet closing failed with wrong faucet balance")
        exit(1)
    
    if(not client.balance("//Bob") == balance_bob + balance(3000)):
        print(f"Faucet closing failed with wrong bob balance")
        exit(1)
    print('Faucet closed', flush=True)

@click.command()
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain.')
@click.option('-p', '--port', default='9944', help='ws-port of the chain.')
@click.option('-l', '--ipfs-local', is_flag=True, help='if set, local ipfs node is used.')
@click.option('-s', '--spec-file', default=f'{TEST_DATA_DIR}{TEST_LOCATIONS_MEDITERRANEAN}', help='Specify community spec-file to be registered.')
@click.option('-t', '--test', is_flag=True, help='if set, run integration tests.')
def main(ipfs_local, client, url, port, spec_file, test):
    client = Client(rust_client=client, node_url=url, port=port)
    cid = create_community(client, spec_file, ipfs_local)

    newbie = client.create_accounts(1)[0]
    faucet(client, cid, [account3, newbie])

    register_participants_and_perform_meetup(client, cid, accounts)

    balance = client.balance(account1)

    print("Claiming early rewards")
    claim_rewards(client, cid, account1)

    if(not balance == client.balance(account1)):
        print("claim_reward fees were not refunded if paid in native currency")
        exit(1)

    client.next_phase()
    client.await_block(1)

    if(not test):
        print(f"Community {cid} successfully bootstrapped")
        return(0)

    print(f'Balances for new community with cid: {cid}.')
    bal = [client.balance(a, cid=cid) for a in accounts]
    [print(f'Account balance for {ab[0]} is {ab[1]}.') for ab in list(zip(accounts, bal))]

    if not round(bal[0]) > 0:
        print("balance is wrong")
        exit(1)

    rep = client.reputation(account1)
    print(rep)
    if not len(rep) > 0:
        print("no reputation gained")
        exit(1)
        
    register_participants_and_perform_meetup(client, cid, accounts)
    client.next_phase()
    client.await_block(1)
    claim_rewards(client, cid, account1, pay_fees_in_cc=True)
    balance1 = client.balance(account1, cid=cid)
    balance2 = client.balance(account2, cid=cid)
    if(not balance1 == balance2):
        print("claim_reward fees were not refunded if paid in cc")
        exit(1)

    test_faucet(client, cid)

    fee_payment_transfers(client, cid)

    test_reputation_caching(client, cid, accounts)

    test_unregister_and_upgrade_registration(client, cid)

    test_endorsements_by_reputables(client, cid)

    print("tests passed")


if __name__ == '__main__':
    main()
