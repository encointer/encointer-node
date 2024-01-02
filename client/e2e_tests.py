#!/usr/bin/env python3

from py_client.client import Client
from lib import *
import time
import os
import subprocess
import signal


class TestError(Exception):
    pass


def run_chain():
    proc = subprocess.Popen('../target/release/encointer-node-notee --dev --enable-offchain-indexing true -lencointer=debug,parity_ws=warn --rpc-port 9945',
                            shell=True, preexec_fn=os.setsid, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT)

    time.sleep(3)
    return proc.pid


def setup_community(client):
    cid = create_community(
        client, spec_file_path=f'{TEST_DATA_DIR}{TEST_LOCATIONS_MEDITERRANEAN}', ipfs_local=True)
    newbie = client.create_accounts(1)[0]
    faucet(client, cid, [account3, newbie])
    register_participants_and_perform_meetup(client, cid, accounts)
    claim_rewards(client, cid, account1)
    client.next_phase()
    client.await_block(1)
    return cid


def kill_process(pid):
    os.killpg(os.getpgid(pid), signal.SIGTERM)


def e2e_test(function):
    def wrapper():
        name = function.__name__
        print(f'Running test: {name}\n')
        pid = run_chain()
        try:
            client = Client(rust_client='../target/release/encointer-client-notee',
                            node_url='ws://127.0.0.1', port='9945')
            cid = setup_community(client)
            function(client, cid)
        except Exception as e:
            kill_process(pid)
            raise e
        kill_process(pid)
        print(f'\nTest success: {name}\n\n')
    return wrapper


@e2e_test
def fee_payment_transfers(client, cid):
    print(f'Transferring 0.5CC from //Alice to //Eve')
    client.transfer(cid, '//Alice', '//Eve', '0.5', pay_fees_in_cc=False)

    print(f'Transferring all CC from //Eve to //Ferdie')
    client.transfer_all(cid, '//Eve', '//Ferdie', pay_fees_in_cc=True)
    if client.balance('//Eve', cid=cid) > 0 or client.balance('//Ferdie', cid=cid) == 0:
        print("transfer_all failed")
        exit(1)


@e2e_test
def test_reputation_caching(client, cid):
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
    print(
        f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
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


@e2e_test
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


@e2e_test
def test_endorsements_by_reputables(client, cid):
    newbies = client.create_accounts(7)
    faucet(client, cid, newbies)

    register_participants_and_perform_meetup(
        client, cid, accounts + newbies[:1])
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


@e2e_test
def test_faucet(client, cid):
    client.set_faucet_reserve_amount("//Alice", balance(3000))
    client.await_block(2)
    balance_bob = client.balance("//Bob")
    client.create_faucet("//Bob", "TestFaucet", balance(10000),
                         balance(1000), [cid], cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    faucet_account = "5CRaq3MpDT1j1d7xoaG3LDwqgC5AoTzRtGptSHm2yFrWoVid"
    print(client.balance("//Bob"), flush=True)
    print(balance_bob, flush=True)
    print(client.balance(faucet_account), flush=True)
    if (not client.balance(faucet_account) == balance(10000)):
        print(f"Wrong Faucet balance after faucet creation")
        exit(1)
    if (not balance_bob - client.balance("//Bob") == balance(13000)):
        print(f"Wrong Bob balance after faucet creation")
        exit(1)
    print('Faucet created', flush=True)

    balance_charlie = client.balance("//Charlie")
    client.drip_faucet("//Charlie", faucet_account, 1,
                       cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    if (not client.balance("//Charlie") == balance_charlie + balance(1000)):
        print(f"Drip failed")
        exit(1)
    print('Faucet dripped', flush=True)

    balance_bob = client.balance("//Bob")
    client.dissolve_faucet("//Alice", faucet_account, "//Eve")
    client.await_block(2)

    if (not client.balance("//Eve") == balance(9000)):
        print(f"Dissolve failed")
        exit(1)

    if (not client.balance("//Bob") == balance_bob + balance(3000)):
        print(f"Dissolve failed")
        exit(1)

    print('Faucet dissolved', flush=True)
    client.create_faucet("//Bob", "TestFaucet", balance(10000),
                         balance(9000), [cid], cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    if (not client.balance(faucet_account) == balance(10000)):
        print(f"Faucet creation failed")
        exit(1)
    print('Faucet created', flush=True)
    client.drip_faucet("//Charlie", faucet_account, 1,
                       cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    print('Faucet dripped', flush=True)
    balance_bob = client.balance("//Bob")
    client.close_faucet("//Bob", faucet_account, cid=cid, pay_fees_in_cc=True)
    client.await_block(2)
    if (not client.balance(faucet_account) == 0):
        print(f"Faucet closing failed with wrong faucet balance")
        exit(1)

    if (not client.balance("//Bob") == balance_bob + balance(3000)):
        print(f"Faucet closing failed with wrong bob balance")
        exit(1)
    print('Faucet closed', flush=True)


@e2e_test
def test_democracy(client, cid):
    print('Starting democracy test...')
    client.next_phase()
    client.next_phase()
    client.next_phase()
    # phase is 9, registering
    print(client.purge_community_ceremony(cid, 1, 8))
    register_participants_and_perform_meetup(client, cid, accounts)
    cindex = 9

    # registering of cindex 10
    client.next_phase()

    claim_rewards(client, cid, "//Alice", pay_fees_in_cc=False)
    client.await_block(1)

    client.next_phase()
    client.next_phase()
    client.next_phase()
    # cindex is now 11

    client.await_block(1)
    client.submit_set_inactivity_timeout_proposal("//Alice", 8)
    client.await_block(1)
    proposals = client.list_proposals()
    print(proposals)
    if ('id: 1' not in proposals):
        print(f"Proposal Submission failed")
        exit(1)

    print('proposal submitted')
    # vote with all reputations gathered so far
    client.vote("//Alice", 1, "aye", [[cid, cindex]])
    client.vote("//Bob", 1, "aye", [[cid, cindex]])
    client.vote("//Charlie", 1, "aye", [[cid, cindex]])

    client.await_block(21)
    client.update_proposal_state("//Alice", 1)
    proposals = client.list_proposals()
    print(proposals)
    if ('Approved' not in proposals):
        print(f"Proposal Voting and Approval failed")
        exit(1)


@e2e_test
def test_balances(client, cid):
    print(f'Balances for new community with cid: {cid}.')
    bal = [client.balance(a, cid=cid) for a in accounts]
    [print(f'Account balance for {ab[0]} is {ab[1]}.')
     for ab in list(zip(accounts, bal))]

    if not round(bal[0]) > 0:
        raise TestError("balance is wrong")

    rep = client.reputation(account1)
    print(rep)
    if not len(rep) > 0:
        raise TestError("no reputation gained")

    register_participants_and_perform_meetup(client, cid, accounts)
    client.next_phase()
    client.await_block(1)
    claim_rewards(client, cid, account1, pay_fees_in_cc=True)
    balance1 = client.balance(account1, cid=cid)
    balance2 = client.balance(account2, cid=cid)
    if (not balance1 == balance2):
        raise TestError("claim_reward fees were not refunded if paid in cc")


def run_tests():

    test_balances()

    exit(0)
    test_faucet(client, cid)

    fee_payment_transfers(client, cid)

    test_reputation_caching(client, cid, accounts)

    test_unregister_and_upgrade_registration(client, cid)

    test_endorsements_by_reputables(client, cid)

    test_democracy(client, cid)

    print("tests passed")


if __name__ == '__main__':
    run_tests()
