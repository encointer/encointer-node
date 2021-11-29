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

global COUNT
COUNT = 0


@click.command()
@click.option('--node_url', default=None, help='if set, remote chain is used with port 443, no need to manually set port, it will be ignored')
@click.option('--client', default='../target/release/encointer-client-notee', help='the client to communicate with the chain')
@click.option('--port', default='9944', help='port for the client to communicate with chain')
def main(node_url, client, port):
    localhost = None
    if node_url is None:
        client = Client(rust_client=client, port=port)
        localhost = "ws://127.0.0.1"
    else:
        client = Client(rust_client=client, node_url='wss://gesell.encointer.org', port=443)
    global COUNT
    with open('typedefs.json') as f:
        custom_type_registry = json.load(f)
    substrate = substrateinterface.SubstrateInterface(
        url=  f"ws://127.0.0.1:{port}" if localhost is not None else f"{node_url}:{443}",
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
