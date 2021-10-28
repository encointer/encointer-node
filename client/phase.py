#!/usr/bin/env python3
"""
encointer ceremony phase controller

Will observe the blockchain events and forward the ceremony phase whenever N blocks were idle

useful for benchmarking bot communities in a local setup
"""

import subprocess
import argparse
import substrateinterface
import json
from py_client.client import Client
from py_client.arg_parser import simple_parser

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
    p = argparse.ArgumentParser(
    prog='register-businesses', parents=[simple_parser()])
    args = p.parse_args()
    # print(f"Starting script with client '{args.client}' on port {args.port}")
    localhost = None
    if(args.node_url == None):
        client = Client(rust_client=args.client, port=args.port)
        localhost = "ws://127.0.0.1"
    else:
        client = Client(rust_client=args.client, node_url='wss://gesell.encointer.org', port=443)
    COUNT = 0
    with open('typedefs.json') as f:
        custom_type_registry = json.load(f)
    substrate = substrateinterface.SubstrateInterface(
        url=  f"ws://127.0.0.1:{args.port}" if localhost != None else f"{args.node_url}:{443}",
        ss58_format=42,
        type_registry_preset='substrate-node-template',
        type_registry=custom_type_registry
    )
    while True:
        result = substrate.query("System", "EventCount", subscription_handler=subscription_handler)
        print('NEXT PHASE!')
        client.next_phase()
        COUNT = 0
