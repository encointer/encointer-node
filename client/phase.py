#!/usr/bin/env python3
"""
encointer ceremony phase controller

Will observe the blockchain events and forward the ceremony phase whenever N blocks were idle

useful for benchmarking bot communities in a local setup
"""

import subprocess
import substrateinterface
import json
from py_client.client import Client

global COUNT


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
    COUNT = 0
    with open('typedefs.json') as f:
        custom_type_registry = json.load(f)
    substrate = substrateinterface.SubstrateInterface(
        url="ws://127.0.0.1:9944",
        ss58_format=42,
        type_registry_preset='substrate-node-template',
        type_registry=custom_type_registry
    )
    client = Client()
    while True:
        result = substrate.query("System", "EventCount", subscription_handler=subscription_handler)
        print('NEXT PHASE!')
        client.next_phase()
        COUNT = 0
