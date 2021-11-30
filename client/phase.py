#!/usr/bin/env python3
"""
encointer ceremony phase controller

Will observe the blockchain events and forward the ceremony phase whenever N blocks were idle

useful for benchmarking bot communities in a local setup
"""

import subprocess
import click
import substrateinterface
import json
from py_client.client import Client
from py_client.helpers import set_local_or_remote_chain

global COUNT
COUNT = 0


@click.command()
@click.option('-r', '--remote_chain', default=None, help='choose one of the remote chains: gesell.')
@click.option('--client', default='../target/release/encointer-client-notee', help='Client binary to communicate with the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
def main(remote_chain, client, port):
    localhost = None
    client = set_local_or_remote_chain(client, port, remote_chain)
    global COUNT
    with open('typedefs.json') as f:
        custom_type_registry = json.load(f)
    substrate = substrateinterface.SubstrateInterface(
        url= f"wss://gesell.encointer.org:{443}" if localhost is not None else f"ws://127.0.0.1:{port}",
        ss58_format=42,
        type_registry_preset='substrate-node-template',
        type_registry=custom_type_registry
    )
    while True:
        result = substrate.query("System", "EventCount", subscription_handler=subscription_handler)
        print('NEXT PHASE!')
        client.next_phase()
        COUNT = 0


def subscription_handler(event_count, update_nr, subscription_id):
    global COUNT
    print(f'events: {event_count}, idle blocks {COUNT}')
    if COUNT > 10:
        return update_nr
    elif event_count.value == 1:
        COUNT += 1
    else:
        COUNT = 0


if __name__ == '__main__':
    main()
