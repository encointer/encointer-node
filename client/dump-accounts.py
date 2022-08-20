
#!/usr/bin/env python3
"""
dump all existing accounts for an endpoint/chain 
"""
from substrateinterface import SubstrateInterface

substrate = SubstrateInterface(
    url="wss://kusama.api.encointer.org"
)

result = substrate.query_map('System', 'Account')

for account, identity_info in result:
    print(f"{account.value}")