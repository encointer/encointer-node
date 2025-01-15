#!/usr/bin/env python3
"""
encointer ceremony phase controller

Will observe the blockchain events and forward the ceremony phase whenever N blocks were idle

useful for benchmarking bot communities in a local setup
"""

import subprocess
import click
from substrateinterface import SubstrateInterface
import json
from py_client.client import Client
from py_client.helpers import set_local_or_remote_chain

global COUNT
COUNT = 0

# a solochain has timestamp.set event in every block, a parachain additionaly has parachainSystem.setValidationData
INTRINSIC_EVENTS = 2

global patience


@click.command()
@click.option('--client', default='../target/release/encointer-client-notee',
              help='Client binary to communicate with the chain.')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('--idle-blocks', default=10, help='how many idle blocks to await before moving to next phase')
def main(client, url, port, idle_blocks):
    client = set_local_or_remote_chain(client, port, url)
    global COUNT, patience
    patience = idle_blocks
    with open('typedefs.json') as f:
        custom_type_registry = json.load(f)
    substrate = SubstrateInterface(
        url=get_node_url(node_url=url, port=port),
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
    global COUNT, patience
    print(f'events: {event_count}, idle blocks {COUNT}')
    if COUNT > patience:
        return update_nr
    elif event_count.value <= INTRINSIC_EVENTS:
        COUNT += 1
    else:
        COUNT = 0


def get_node_url(node_url, port):
    if node_url == "gesell":
        return f"wss://gesell.encointer.org:{443}"
    else:
        return f"{node_url}:{port}"


if __name__ == '__main__':
    main()
