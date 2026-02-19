import requests
import pandas as pd

# Define the URL and the data for the POST request
url = "https://api.encointer.org/v1/indexer2/query"
data = {
    "collection": "extrinsics",
    "query": {
        "section": "encointerFaucet",
        "method": "drip",
        "success": True,
    },
    "options": {
        "limit": 999,
        "skip": 0,
        "sort": {
            "blockNumber": -1
        }
    }
}

# Send the POST request
response = requests.post(url, json=data)

# Check if the request was successful
if response.status_code == 200:
    # Parse the JSON response into a pandas DataFrame
    json_response = response.json()
    data = []
    for item in json_response:
        signer = item['signer']['Id']
        extrinsic_id = item['_id']
        block_hash = item['blockHash']
        faucet = item['args']['faucet_account']
        cid = item['args']['cid']
        cindex = item['args']['cindex']
        print(f"{extrinsic_id}, {block_hash}, {signer}, {faucet}, {cid}, {cindex}")
    # Print the DataFrame
else:
    print(f"Request failed with status code {response.status_code}")
