#!/usr/bin/env python3
"""
Demonstrate the bootstrapping of an Encointer community on a *dev* chain.

start node with
  ../target/release/encointer-node --dev --tmp --ws-port 9945 --enable-offchain-indexing true --rpc-methods unsafe

or start parachain with
then run this script
  ./bootstrap_demo_community.py --port 9945

"""

import json
import os
import time
import click

from py_client.client import Client
from py_client.scheduler import CeremonyPhase
from py_client.ipfs import Ipfs, ASSETS_PATH
from py_client.pool import create_substrate_connection, wait_for_pool_empty

account1 = '//Alice'
account2 = '//Bob'
account3 = '//Charlie'
accounts = [account1, account2, account3]

TEST_DATA_DIR = './test-data/'
TEST_LOCATIONS_MEDITERRANEAN = 'test-locations-mediterranean.json'


def check_participant_count(client, cid, type, number):
    print(f"🔎 Checking if number of registered participants for type {type} == {number}")
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
    if (str(cindex), cid, reputation) not in rep:
        print(f"🔎 Reputation for {account} in cid {cid} cindex {cindex} is not {reputation}")
        exit(1)


def perform_meetup(client, cid, accounts):
    print('🫂 Starting meetup...')

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


def create_community(client, spec_file_path, ipfs_local, signer, wrap_call="none", batch_size=100):
    # non sudoer can create community on gesell (provide --signer but don't use //Alice), but not on parachain (where council will create)
    cid = client.new_community(spec_file_path, signer=signer, wrap_call=wrap_call, batch_size=batch_size)
    if len(cid) > 10:
        print(f'👬 Registered community with cid: {cid}')
    else:
        exit(1)

    print('Uploading assets to ipfs')
    root_dir = os.path.realpath(ASSETS_PATH)
    ipfs_cid = Ipfs.add_recursive(root_dir, ipfs_local)

    print(f'Updating Community spec with ipfs cid: {ipfs_cid}')
    update_spec_with_cid(spec_file_path, ipfs_cid)

    return cid


def register_participants_and_perform_meetup(client, cid, accounts, substrate):
    print(client.list_communities())
    client.go_to_phase(CeremonyPhase.Registering)

    print(f'📝 Registering Participants for cid: {cid}')
    [client.register_participant(b, cid) for b in accounts]

    print("⌛ Waiting for tx pool to drain...")
    wait_for_pool_empty(substrate)

    print(client.list_participants(cid))
    client.go_to_phase(CeremonyPhase.Assigning)

    print('Listing meetups')
    print(client.list_meetups(cid))
    client.go_to_phase(CeremonyPhase.Attesting)

    print(f'Performing meetups for cid {cid}')
    perform_meetup(client, cid, accounts)

    print("⌛ Waiting for tx pool to drain...")
    wait_for_pool_empty(substrate)

    print(client.list_attestees(cid))


def faucet(client, cid, accounts, substrate):
    # charlie has no genesis funds
    print('✨ native (Alice)Faucet is dripping...')
    client.faucet(accounts, is_faucet=True)

    print("⌛ Waiting for tx pool to drain...")
    wait_for_pool_empty(substrate)


def test_fee_payment_transfers(client, cid, substrate):
    print(f'🔄 Transferring 0.5CC from //Alice to //Eve')
    client.transfer(cid, '//Alice', '//Eve', '0.5', pay_fees_in_cc=False)
    wait_for_pool_empty(substrate)

    print(f'🔄 Transferring all CC from //Eve to //Ferdie')
    client.transfer_all(cid, '//Eve', '//Ferdie', pay_fees_in_cc=True)

    wait_for_pool_empty(substrate)
    if client.balance('//Eve', cid=cid) > 0 or client.balance('//Ferdie', cid=cid) == 0:
        print("transfer_all failed")
        exit(1)


def claim_rewards(client, cid, account, meetup_index=None, all=False, pay_fees_in_cc=False):
    print("🏆 Claiming rewards")
    client.claim_reward(account, cid, meetup_index=meetup_index, all=all, pay_fees_in_cc=pay_fees_in_cc)


def test_reputation_caching(client, cid, substrate):
    """ This test assumes that one successful ceremony has been run before. """
    print('################## Testing reputation caching...')
    register_participants_and_perform_meetup(client, cid, accounts, substrate)
    client.go_to_phase(CeremonyPhase.Registering)
    # query reputation to set the cache in the same phase as claiming rewards
    # so we would have a valid cache value, but the cache should be invalidated
    # anyways because of the dirty bit
    client.reputation(account1)
    claim_rewards(client, cid, account1)
    wait_for_pool_empty(substrate)

    # check if the reputation cache was updated
    rep = client.reputation(account1)
    cindex = client.get_cindex()

    print(f'Reputations of account1: {rep}')

    if (f'{cindex -2}', 'sqm1v79dF6b', f'VerifiedLinked({cindex-1})') not in rep:
        print('Error: Last reputation has not been linked.')
        exit(1)

    if (f'{cindex-1}', 'sqm1v79dF6b', 'VerifiedUnlinked') not in rep:
        print('Error: Did not receive reputation.')
        exit(1)


    # test if reputation cache is invalidated after registration
    print(f'📝 Registering Participants for Cid: {cid}')
    [client.register_participant(b, cid) for b in accounts]

    print("⌛ Waiting for tx pool to drain...")
    wait_for_pool_empty(substrate)

    rep = client.reputation(account1)
    print(f'Reputations of account1: {rep}')
    # after the registration the second reputation should now be linked
    if (f'{cindex-1}', 'sqm1v79dF6b', f'VerifiedLinked({cindex})') not in rep:
        print("Error: new reputation has not been linked")
        exit(1)

    client.go_to_phase(CeremonyPhase.Assigning)
    client.go_to_phase(CeremonyPhase.Attesting)
    client.go_to_phase(CeremonyPhase.Registering)

    # check if reputation cache gets updated after phase change
    cindex = client.get_cindex()
    print(client.purge_community_ceremony(cid, 1, cindex))
    wait_for_pool_empty(substrate)

    client.go_to_phase(CeremonyPhase.Assigning)
    rep = client.reputation(account1)
    # after phase change cache will be updated
    if not len(rep) == 0:
        print(f"Error: reputation was not cleared, should be 0, is {len(rep)}")
        exit(1)

    client.go_to_phase(CeremonyPhase.Attesting)
    client.go_to_phase(CeremonyPhase.Registering)
    # registering


def test_unregister_and_upgrade_registration(client, cid, substrate):
    print('################## Testing unregister and upgrade registration...')
    newbie = client.create_accounts(1)[0]
    faucet(client, cid, [newbie], substrate)

    register_participants_and_perform_meetup(client, cid, accounts + [newbie], substrate)
    client.go_to_phase(CeremonyPhase.Registering)
    # registering phase

    client.register_participant(newbie, cid)
    wait_for_pool_empty(substrate)
    print(client.list_participants(cid))
    # before claiming, no rep. therefore still newbie
    check_participant_count(client, cid, "Newbie", 1)

    claim_rewards(client, cid, account1, pay_fees_in_cc=True)
    wait_for_pool_empty(substrate)

    eligible_cindex = client.get_cindex() - 1
    print(f"🔎 checking newbie reputation for cindex {eligible_cindex}")
    check_reputation(client, cid, newbie, eligible_cindex, "VerifiedUnlinked")
    client.upgrade_registration(newbie, cid)
    wait_for_pool_empty(substrate)

    check_participant_count(client, cid, "Newbie", 0)
    check_participant_count(client, cid, "Reputable", 1)

    check_reputation(client, cid, newbie, eligible_cindex, f"VerifiedLinked({eligible_cindex + 1})")

    client.unregister_participant(newbie, cid, cindex=eligible_cindex)
    wait_for_pool_empty(substrate)
    check_participant_count(client, cid, "Reputable", 0)

    check_reputation(client, cid, newbie, eligible_cindex, "VerifiedUnlinked")


def test_endorsements_by_reputables(client, cid, substrate):
    print('################## Testing endorsements by reputables...')
    newbies = client.create_accounts(7)
    faucet(client, cid, newbies, substrate)

    register_participants_and_perform_meetup(client, cid, accounts + newbies[:1], substrate)
    client.go_to_phase(CeremonyPhase.Registering)
    claim_rewards(client, cid, account1, pay_fees_in_cc=True)
    wait_for_pool_empty(substrate)
    # newbies[0] is now reputable
    check_participant_count(client, cid, "Endorsee", 0)

    # endorsement works before registration
    client.endorse_newcomers(cid, newbies[0], [newbies[1]])
    wait_for_pool_empty(substrate)
    client.register_participant(newbies[1], cid)
    wait_for_pool_empty(substrate)
    check_participant_count(client, cid, "Endorsee", 1)

    # endorsement works after registration
    for i in range(2, 6):
        client.register_participant(newbies[i], cid)
        wait_for_pool_empty(substrate)
        client.endorse_newcomers(cid, newbies[0], [newbies[i]])
        wait_for_pool_empty(substrate)

        check_participant_count(client, cid, "Endorsee", i)

    # all tickets used, should fail
    print(client.endorse_newcomers(cid, newbies[0], [newbies[6]]))
    wait_for_pool_empty(substrate)
    # endorsee count is still 5
    check_participant_count(client, cid, "Endorsee", 5)


def balance(x):
    return x * 10 ** 12

def test_first_ceremony_with_early_claim(client, cid, substrate):
    faucet(client, cid, [account3], substrate)

    register_participants_and_perform_meetup(client, cid, accounts, substrate)

    balance = client.balance(account1)

    print("Claiming early rewards")
    claim_rewards(client, cid, account1)
    wait_for_pool_empty(substrate)

    if (not balance == client.balance(account1)):
        print("claim_reward fees were not refunded if paid in native currency")
        exit(1)

    client.go_to_phase(CeremonyPhase.Registering)

    print(f'Balances for new community with cid: {cid}.')
    bal = [client.balance(a, cid=cid) for a in accounts]
    [print(f'Account balance for {ab[0]} is {ab[1]}.') for ab in list(zip(accounts, bal))]

    if not round(bal[0]) > 0:
        print("Did not receive ceremony rewards")
        exit(1)

    rep = client.reputation(account1)
    print(rep)
    if not len(rep) > 0:
        print("no reputation gained")
        exit(1)

    print(f"Community {cid} successfully bootstrapped")

def test_second_ceremony_with_cc_payment_and_regular_claim(client, cid, substrate):
    register_participants_and_perform_meetup(client, cid, accounts, substrate)
    client.go_to_phase(CeremonyPhase.Registering)

    claim_rewards(client, cid, account1, pay_fees_in_cc=True)
    wait_for_pool_empty(substrate)

    balance1 = client.balance(account1, cid=cid)
    balance2 = client.balance(account2, cid=cid)
    if (not balance1 == balance2):
        print("claim_reward fees were not refunded if paid in cc")
        exit(1)


def test_faucet(client, cid, substrate, is_parachain):
    """ First we create a faucet that is closed afterward, and
        then we create another one that is dissolved.
    """
    print("################ Testing the EncointerFaucet....")
    client.set_faucet_reserve_amount("//Alice", balance(3000))
    wait_for_pool_empty(substrate)
    faucet_account = "5CRaq3MpDT1j1d7xoaG3LDwqgC5AoTzRtGptSHm2yFrWoVid"
    eligible_cindex = client.get_cindex() - 1

    client.create_faucet("//Bob", "TestFaucet", balance(10000), balance(9000), [cid], cid=cid, pay_fees_in_cc=True)
    wait_for_pool_empty(substrate)
    if (not client.balance(faucet_account) == balance(10000)):
        print(f"TestFaucet creation failed: Faucet does not have the expected funds")
        exit(1)
    print('Faucet created', flush=True)

    client.drip_faucet("//Charlie", faucet_account, eligible_cindex, cid=cid, pay_fees_in_cc=True)
    wait_for_pool_empty(substrate)
    print('Faucet dripped', flush=True)

    balance_bob = client.balance("//Bob")
    print(f'Bobs balance before closing the faucet: {balance_bob}')
    client.close_faucet("//Bob", faucet_account, cid=cid, pay_fees_in_cc=True)
    wait_for_pool_empty(substrate)
    balance_bob_after_closing = client.balance("//Bob")
    print(f'Bobs balance after closing the faucet: {balance_bob_after_closing}')

    if (not client.balance(faucet_account) == 0):
        print(f"Faucet closing failed: Faucet is not empty")
        exit(1)

    # Ensure Bobs balance increased due to refund of the deposit
    # Todo: Check with exact value the same way as below, but the parachain has a different reserve deposit
    # so we just check that bob received something.
    if (balance_bob_after_closing <= balance_bob):
        print(f"Faucet closing failed: Bob did not receive the reserve deposit")
        exit(1)
    print('Faucet closed', flush=True)


    # Create a second faucet and test that dissolving works

    balance_bob = client.balance("//Bob")
    print(f'Bobs balance before creating the 2nd faucet: {balance_bob}')
    client.create_faucet("//Bob", "TestFaucet", balance(10000), balance(1000), [cid], cid=cid, pay_fees_in_cc=True)
    wait_for_pool_empty(substrate)

    # Should be balance_before - faucet_deposit
    balance_bob_after_creating = client.balance("//Bob")
    print(f'Bobs balance after creating the 2nd faucet: {balance_bob_after_creating}')

    balance_faucet = client.balance(faucet_account)
    print(f'Faucet balance: {balance_faucet}')
    print(balance_faucet, flush=True)
    if (not balance_faucet == balance(10000)):
        print(f"Wrong Faucet balance after faucet creation")
        exit(1)
    print('Faucet created', flush=True)

    balance_charlie = client.balance("//Charlie")
    print(f"Charlie uses participation at cindex: {eligible_cindex} to drip faucet and pays fees in CC")
    client.drip_faucet("//Charlie", faucet_account, eligible_cindex, cid=cid, pay_fees_in_cc=True)
    wait_for_pool_empty(substrate)

    if (not client.balance("//Charlie") == balance_charlie + balance(1000)):
        print(f"Drip failed: Charlie did not receive the drip amount")
        exit(1)
    print('Faucet dripped', flush=True)


    # The parachain uses root instead of council for this, which we don't support yet.
    if is_parachain:
        print("Skip testing dissolving faucet, as the script does not "
              "support dissolving the faucet yet in the parachain case")
        return

    balance_bob = client.balance("//Bob")
    client.dissolve_faucet("//Alice", faucet_account, "//Eve")
    wait_for_pool_empty(substrate)

    if (not client.balance("//Eve") == balance(9000)):
        print(f"Dissolve failed: Eve did not receive the remaining funds")
        exit(1)

    if (not client.balance("//Bob") == balance_bob + balance(3000)):
        print(f"Dissolve failed: Bob did not receive the deposit refund")
        exit(1)

    print('Faucet dissolved', flush=True)


def test_ipfs_upload(client, cid, substrate):
    """Test IPFS upload: CC holders succeed, non-holders get 403."""
    import tempfile
    gateway_url = os.environ.get('IPFS_GATEWAY_URL', 'http://localhost:5050')

    # Create test file
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        f.write('{"test": "data"}')
        test_file = f.name

    # Test 1: //Alice (CC holder) should succeed
    print("Testing //Alice (CC holder)...")
    success, output, code = client.ipfs_upload('//Alice', test_file, cid, gateway_url)
    if not success:
        print(f"ERROR: //Alice upload failed: {output}")
        os.remove(test_file)
        exit(1)
    ipfs_cid = output.strip().split('\n')[-1]
    print(f"//Alice upload succeeded: {ipfs_cid}")

    # Verify uploaded content via ipfs cat
    import subprocess
    ret = subprocess.run(['ipfs', 'cat', ipfs_cid], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if ret.returncode != 0:
        print(f"ERROR: ipfs cat {ipfs_cid} failed: {ret.stderr.decode('utf-8').strip()}")
        os.remove(test_file)
        exit(1)
    fetched = ret.stdout.decode('utf-8')
    with open(test_file) as f:
        expected = f.read()
    if fetched != expected:
        print(f"ERROR: content mismatch. Expected: {expected!r}, got: {fetched!r}")
        os.remove(test_file)
        exit(1)
    print(f"Verified: ipfs cat {ipfs_cid} matches uploaded content")

    # Test 2: //Zoe (no genesis balance) should fail
    print("Testing //Zoe (account does not exist on chain)...")
    success, output, code = client.ipfs_upload('//Zoe', test_file, cid, gateway_url)
    if success:
        print("ERROR: //Zoe should have been rejected")
        os.remove(test_file)
        exit(1)
    if code != 61:  # 61 = NOT_CC_HOLDER exit code
        print(f"ERROR: Expected exit code 61, got {code}")
        os.remove(test_file)
        exit(1)
    print("//Zoe correctly rejected")

    os.remove(test_file)
    print("IPFS upload test passed!")


def test_democracy(client, cid, substrate):
    print("################ Testing democracy ...")
    client.go_to_phase(CeremonyPhase.Assigning)
    client.go_to_phase(CeremonyPhase.Attesting)
    client.go_to_phase(CeremonyPhase.Registering)
    # phase is 9, registering
    print(client.purge_community_ceremony(cid, 1, 8))
    register_participants_and_perform_meetup(client, cid, accounts, substrate)
    eligible_cindex = client.get_cindex()

    # registering of cindex 10
    client.go_to_phase(CeremonyPhase.Registering)

    claim_rewards(client, cid, "//Alice", pay_fees_in_cc=False)
    wait_for_pool_empty(substrate)

    client.go_to_phase(CeremonyPhase.Assigning)
    client.go_to_phase(CeremonyPhase.Attesting)
    client.go_to_phase(CeremonyPhase.Registering)
    # cindex is now 11

    client.submit_set_inactivity_timeout_proposal("//Alice", 8)
    wait_for_pool_empty(substrate)
    proposals = client.list_proposals()
    print(proposals)
    if ('id: 1' not in proposals):
        print(f"Proposal Submission failed")
        exit(1)

    print('proposal submitted')

    print("Alices reputation: " + ' '.join([str(item) for item in client.reputation("//Alice")]))
    print("Bobs reputation: " + ' '.join([str(item) for item in client.reputation("//Bob")]))
    print("Charlies reputation: " + ' '.join([str(item) for item in client.reputation("//Charlie")]))
    print(f"will vote with only cindex {eligible_cindex} reputation")
    # vote with all reputations gathered so far
    client.vote("//Alice", 1, "aye", [[cid, eligible_cindex]])
    client.vote("//Bob", 1, "aye", [[cid, eligible_cindex]])
    client.vote("//Charlie", 1, "aye", [[cid, eligible_cindex]])
    wait_for_pool_empty(substrate)
    proposals = client.list_proposals()
    print("--proposals")
    print(proposals)
    print("--")
    print("⌛ Polling for proposal approval (confirmation phase ~5min)...")
    deadline = time.monotonic() + 360
    while time.monotonic() < deadline:
        client.update_proposal_state("//Alice", 1)
        wait_for_pool_empty(substrate)
        proposals = client.list_proposals()
        if 'Approved' in proposals or 'Enacted' in proposals:
            break
        time.sleep(10)
    else:
        print("--proposals")
        print(proposals)
        print("--")
        print(f"Proposal Voting and Approval failed")
        exit(1)

    print("--proposals")
    print(proposals)
    print("--")


@click.command()
@click.option('--client', default='../target/release/encointer-cli',
              help='Client binary to communicate with the chain.')
@click.option('--signer', help='optional account keypair creating the community')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain, or `gesell` alternatively.')
@click.option('-p', '--port', default='9944', help='ws-port of the chain.')
@click.option('-l', '--ipfs-local', is_flag=True, help='if set, local ipfs node is used.')
@click.option('-s', '--spec-file', default=f'{TEST_DATA_DIR}{TEST_LOCATIONS_MEDITERRANEAN}',
              help='Specify community spec-file to be registered.')
@click.option('-t', '--test', default="none", help='Define if/which integration tests should be run')
@click.option('-w', '--wrap-call', default="none", help='wrap the call, values: none|sudo|collective')
@click.option('-b', '--batch-size', default=100, help='batch size of the addLocation call (parachain is limited to 15)')
@click.option('--is-parachain', is_flag=True, help='If the connecting chain is a parachain')
def main(ipfs_local, client, signer, url, port, spec_file, test, wrap_call, batch_size, is_parachain):
    print(f"Chain is-parchain: {is_parachain}")

    substrate = create_substrate_connection(node_url=url, port=int(port))
    client = Client(rust_client=client, node_url=url, port=port)
    cid = create_community(client, spec_file, ipfs_local, signer, wrap_call=wrap_call, batch_size=batch_size)

    # Strategy: run the first ceremony to ensure that some CC have been minted,
    # and then do the individual tests.
    test_first_ceremony_with_early_claim(client, cid, substrate)

    match test:
        case "none":
            return 0
        case "cc-fee-payment":
            test_second_ceremony_with_cc_payment_and_regular_claim(client, cid, substrate)
            test_fee_payment_transfers(client, cid, substrate)
        case "faucet":
            test_faucet(client, cid, substrate, is_parachain)
        case "reputation-caching":
            # Fixme: fails for parachain as purging community ceremony requires root
            test_reputation_caching(client, cid, substrate)
        case "unregister-and-upgrade-registration":
            test_unregister_and_upgrade_registration(client, cid, substrate)
        case "endorsement":
            test_endorsements_by_reputables(client, cid, substrate)
        case "democracy":
            # Fixme: democracy params are runtime constants, and therefore we can't test it with the parachain.
            test_democracy(client, cid, substrate)
        case "ipfs-upload":
            test_ipfs_upload(client, cid, substrate)
        case _:
            return "Invalid value for test"

    print("tests passed")


if __name__ == '__main__':
    main()
