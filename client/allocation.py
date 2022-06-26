#!/usr/bin/env python3
#
# reads allocations from a csv file in [ksm] and writes batch extrinsics to a .hex file
import csv
import os

import click

from math import floor
from typing import IO

from substrateinterface import SubstrateInterface
from substrateinterface.utils.ss58 import is_valid_ss58_address, ss58_decode, ss58_encode
from py_client.communities import random_community_spec, COMMUNITY_SPECS_PATH
from py_client.helpers import purge_prompt, read_cid, write_cid, set_local_or_remote_chain
from py_client.client import Client, ExtrinsicFeePaymentImpossible, ExtrinsicWrongPhase, UnknownError, \
    ParticipantAlreadyLinked
# Settings:


# our chain uses this format
default_ss58_format = 2

cid = "u0qj9QqA2Q"
rescue = "5HpZeMmqbQ1Rmbfrko3xopr9iaPFwiGLB5p7VN7KSYkgTWoE"
substrate = None

@click.command()
@click.argument('filename')
@click.option('--endpoint', default="ws://localhost:9944", help='rpc websocket endpoint to talk to the chain')
@click.option('--client', default='../target/release/encointer-client-notee',
              help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws(s)-port of the chain.')
@click.option('--verify', is_flag=True, help='only verify balances')
def allocate(filename, endpoint, client, port, verify):
    global substrate
    substrate = SubstrateInterface(
        url=endpoint
    )

    api = Client(rust_client=client, node_url=endpoint, port=port)

    count = 0
    beneficiaries = []
    allocation_circulating = 0
    allocation_unvested = 0

    if verify:
        logfilename = filename + ".verify.log"
    else:
        logfilename = filename + ".log"

    rescue_balance_initial = api.balance(rescue, cid=cid)
    print(f"rescue account has {rescue_balance_initial}")
    with open(filename) as allocation_file, open(logfilename, "w") as log_file:
        reader = csv.reader(allocation_file)
        for row in reader:
            # empty lines
            if (len(row) == 0):
                continue
            # skip comments
            if (row[0].strip()[:1] == '#'):
                continue

            target_address = row[0]
            try:
                target_balance = float(row[2])
            except ValueError:
                print(f"ERROR: cannot read {target_address}'s target balance from csv")
                continue
            print(f"{target_address} target balance: {target_balance}")

            # ensure the address is valid and convert it to our own ss58format if necessary
            try:
                pubkey = ss58_decode(target_address)
                target_address_ksm = ss58_encode(pubkey, ss58_format=default_ss58_format)
                conversion_info = '';
            except ValueError:
                print(f"ERROR: account {target_address} not valid. ignoring")
            if target_address != target_address_ksm:
                print(f"converted {target_address} to {target_address_ksm}")
                conversion_info=f"({target_address})"

            if target_address_ksm in beneficiaries:
                log_file.write(f"WARNING: account {target_address_ksm} {conversion_info} appears multiple times\n")
            beneficiaries.append(target_address_ksm)

            # Check the balance onchain
            free = api.balance(target_address_ksm, cid=cid)
            print(f"has balance: {free}")
            if free > 0.0:
                log_file.write(f"[WARNING] account {target_address_ksm} {conversion_info} already has {free}. skipping\n")
            else:
                log_file.write(f"account {target_address_ksm} {conversion_info} gets {target_balance}\n")
                print(f"sending({count}): {target_balance}")
                api.transfer(cid, rescue, target_address_ksm, str(target_balance), pay_fees_in_cc=True)
                allocation_circulating += target_balance
                count += 1
                print(f"sent: {target_balance}")




        print(f" >> all {count} calls generated << ")
        print(f"circulating allocation {allocation_circulating}")


if __name__ == '__main__':
    allocate()
