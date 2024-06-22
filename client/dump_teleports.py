import requests
import pandas as pd

# Define the URL and the data for the POST request
url = "https://api.encointer.org/v1/indexer2/query"
data = {
    "collection": "extrinsics",
    "query": {
        "section": "polkadotXcm",
        "method": "limitedTeleportAssets"
    },
    "options": {
        "limit": 100,
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
        beneficiary = ''
        if 'V1' in item['args']['beneficiary']:
            beneficiary = item['args']['beneficiary']['V1']['interior']['X1']['AccountId32']['id']
        elif 'V3' in item['args']['beneficiary']:
            beneficiary = item['args']['beneficiary']['V3']['interior']['X1']['AccountId32']['id']
        elif 'V4' in item['args']['beneficiary']:
            beneficiary = item['args']['beneficiary']['V4']['interior']['X1'][0]['AccountId32']['id']

        amount = 0
        if 'V1' in item['args']['assets']:
            amount = int(item['args']['assets']['V1'][0]['fun']['Fungible'].replace(',', ''))
        elif 'V3' in item['args']['beneficiary']:
            amount = int(item['args']['assets']['V3'][0]['fun']['Fungible'].replace(',', ''))
        elif 'V4' in item['args']['beneficiary']:
            amount = int(item['args']['assets']['V4'][0]['fun']['Fungible'].replace(',', ''))
        print(f"{extrinsic_id}, {block_hash}, {signer}, {beneficiary}, {amount}")
    # Print the DataFrame
else:
    print(f"Request failed with status code {response.status_code}")
