#!/usr/bin/env python3
"""
encointer ceremony phase controller

Will observe the blockchain events and forward the ceremony phase whenever N blocks were idle

useful for benchmarking bot communities in a local setup
"""

import subprocess
import time
import click
from substrateinterface import SubstrateInterface
import json
from py_client.client import Client
from py_client.helpers import set_local_or_remote_chain

global COUNT, patience, armed
COUNT = 0
armed = False

# a solochain has timestamp.set event in every block, a parachain additionaly has parachainSystem.setValidationData and
# a balance transfer for the collator rewards.
INTRINSIC_EVENTS = 3


@click.command()
@click.option('--client', default='../target/release/encointer-client-notee',
              help='Client binary to communicate with the chain.')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain, or `gesell` alternatively.')
@click.option('--port', default='9944', help='ws-port of the chain.')
@click.option('--idle-blocks', default=10, help='how many idle blocks to await before moving to next phase')
def main(client, url, port, idle_blocks):
    client = set_local_or_remote_chain(client, port, url)
    global COUNT, patience, armed
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
        for attempt in range(5):
            try:
                client.next_phase()
                break
            except Exception as e:
                print(f'next_phase attempt {attempt + 1} failed: {e}, retrying in 6s...')
                time.sleep(6)
        COUNT = 0


def subscription_handler(event_count, update_nr, subscription_id):
    global COUNT, patience, armed
    print(f'events: {event_count}, idle blocks {COUNT}, armed: {armed}')
    if event_count.value > INTRINSIC_EVENTS:
        armed = True
        COUNT = 0
    elif armed:
        COUNT += 1
        if COUNT > patience:
            return update_nr


def get_node_url(node_url, port):
    if node_url == "gesell":
        return f"wss://gesell.encointer.org:{443}"
    else:
        return f"{node_url}:{port}"


if __name__ == '__main__':
    main()
