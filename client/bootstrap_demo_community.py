#!python
from py_client.client import Client
from py_client.scheduler import CeremonyPhase

cli = ["../target/release/encointer-client.py-notee -p 9444"]

account1 = '//Alice'
account2 = '//Bob'
account3 = '//Charlie'
accounts = [account1, account2, account3]


def perform_meetup(client, cid):
    print('Starting meetup...')
    print('Creating claims...')
    vote = len(accounts)
    claim1 = client.new_claim(account1, vote, cid)
    claim2 = client.new_claim(account2, vote, cid)
    claim3 = client.new_claim(account3, vote, cid)

    print('Signing claims...')
    witness1_2 = client.sign_claim(account1, claim2)
    witness1_3 = client.sign_claim(account1, claim3)

    witness2_1 = client.sign_claim(account2, claim1)
    witness2_3 = client.sign_claim(account2, claim3)

    witness3_1 = client.sign_claim(account3, claim1)
    witness3_2 = client.sign_claim(account3, claim2)

    print('Sending witnesses to chain...')
    client.register_attestations(account1, [witness2_1, witness3_1])
    client.register_attestations(account2, [witness1_2, witness3_2])
    client.register_attestations(account3, [witness1_3, witness2_3])


def main(client=Client()):
    cid = client.new_community('test-locations-mediterranean.json')
    print(f'Registered community with cid: {cid}')

    print(client.list_communities())
    client.go_to_phase(CeremonyPhase.REGISTERING)

    # charlie has no genesis funds
    print('Faucet is dripping to Charlie...')
    client.faucet([account3])

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print('Registering Participants...')
    [client.register_participant(b, cid) for b in accounts]

    blocks_to_wait = 3
    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print(client.list_participants(cid))
    client.next_phase()

    print('Listing meetups')
    print(client.list_meetups(cid))
    client.next_phase()

    perform_meetup(client, cid)

    print(f"Waiting for {blocks_to_wait} blocks, such that xt's are processed...")
    client.await_block(blocks_to_wait)

    print(client.list_attestations(cid))
    client.next_phase()

    print(f'Balances for new community with cid: {cid}.')
    bal = [client.balance(a, cid=cid) for a in accounts]
    [print(f'Account balance for {ab[0]} is {ab[1]}.') for ab in list(zip(accounts, bal))]

    if round(bal[0]) > 0:
        print("tests passed")
    else:
        print("balance is wrong")
        exit(1)


if __name__ == '__main__':
    main()
