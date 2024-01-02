import json
import os
from py_client.scheduler import CeremonyPhase
from py_client.ipfs import Ipfs, ASSETS_PATH

account1 = '//Alice'
account2 = '//Bob'
account3 = '//Charlie'
accounts = [account1, account2, account3]

TEST_DATA_DIR = './test-data/'
TEST_LOCATIONS_MEDITERRANEAN = 'test-locations-mediterranean.json'


def claim_rewards(client, cid, account, meetup_index=None, all=False, pay_fees_in_cc=False):
    print("Claiming rewards")
    client.claim_reward(account, cid, meetup_index=meetup_index,
                        all=all, pay_fees_in_cc=pay_fees_in_cc)
    client.await_block(1)


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
        print(
            f"Reputation for {account} in cid {cid} cindex {cindex} is not {reputation}")
        exit(1)


def perform_meetup(client, cid, accounts):
    print('Starting meetup...')

    print('Attest other attendees...')
    for account in accounts:
        client.attest_attendees(
            account, cid, [a for a in accounts if a != account])


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

    blocks_to_wait = 1
    print(
        f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print(client.list_participants(cid))
    client.next_phase()

    print('Listing meetups')
    print(client.list_meetups(cid))
    client.next_phase()

    print(f'Performing meetups for cid {cid}')
    perform_meetup(client, cid, accounts)

    print(
        f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print(client.list_attestees(cid))


def faucet(client, cid, accounts):
    # charlie has no genesis funds
    print('Faucet is dripping...')
    client.faucet(accounts, is_faucet=True)

    blocks_to_wait = 1
    print(
        f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)
