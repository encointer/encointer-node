import subprocess
import substrateinterface
import json
from py_client.client import Client

global COUNT


def subscription_handler(event_count, update_nr, subscription_id):
    global COUNT
    print(event_count, COUNT)
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
