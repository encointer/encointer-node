#!/usr/bin/env python3
"""
create a substrate batch call from a file with individual calls per line, hex encoded and 0x prefixed
"""

from substrateinterface import SubstrateInterface
from substrateinterface.base import RuntimeConfigurationObject
from scalecodec.types import GenericCall
from scalecodec import ScaleBytes
import click
import csv

@click.command()
@click.argument('filename')
@click.option('--endpoint', default="ws://localhost:9944", help='rpc websocket endpoint to talk to the chain')
@click.option('--maxcalls', default=100, help='maximum number of calls in same batch')
def batch(filename, endpoint, maxcalls):
    substrate = SubstrateInterface(
        url=endpoint
    )
    #call = RuntimeConfigurationObject().create_scale_object("Call")
    calls = []
    with open(filename) as calls_file:
        reader = csv.reader(calls_file)
        for row in reader:
            call = GenericCall(ScaleBytes(row[0]))
            calls.append(call)
    
    print(f"read {len(calls)} calls from {filename}")

    chunks = [calls[i:i + maxcalls] for i in range(0, len(calls), maxcalls)]
    for chunk in chunks:
        batch_call = substrate.compose_call(
            call_module='Utility',
            call_function='batch_all',
            call_params={
                'calls': chunk
            }
        )
        print(batch_call.encode())

if __name__ == '__main__':
    batch()