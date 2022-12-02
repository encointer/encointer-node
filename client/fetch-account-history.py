#!/usr/bin/env python3
#
# ./fetch-account-history.py <subscan api key>
#
import requests
import csv
import sys
import json
import time
from datetime import datetime
from substrateinterface.utils.ss58 import ss58_decode, ss58_encode
from base58 import b58encode

account = "all"
api_key = sys.argv[1]

page_rows = 100

t_start = time.time()

start_block = 1689688 # 1187284 #781981 #535620
end_block = 1690155 #660815
blocks_total = end_block - start_block

def decode_cid(cid_raw):
    geohash = cid_raw['geohash']
    digest_raw = cid_raw['digest']
    if digest_raw[0:1] == '0x':
        digest = b58encode(bytearray.fromhex(digest_raw[2:])).decode("utf-8")
    else:
        digest = digest_raw
    return geohash + digest


with open(f'account-events-{account}-{start_block}_to_{end_block}.csv', 'w', newline='') as csvfile:
    writer = csv.writer(csvfile, delimiter=',')

    for block in range(start_block, end_block):
        page = 0
        while True:
            try:
                response = requests.post('https://encointer.api.subscan.io/api/scan/events',
                                     headers={
                                         'Content-Type': 'application/json',
                                         'X-API-Key': api_key,
                                         'Accept': 'application/json',
                                     },
                                     json={
                                         'row': 100,
                                         'page': page,
                                         'module': 'encointerBalances',
                                         'block_num': block
                                     }
                                     )
                events = response.json()['data']['events']
            except:
                print(response)
                try:
                    print(response.json())
                except:
                    print("error decoding response")
                print("sleeping a bit")
                time.sleep(0.9)
                continue

            remaining = int(response.headers['RateLimit-Remaining'])
            if remaining < 1:
                print(f"approaching rate limit {remaining}/{response.headers['RateLimit-Limit']}")




            progress = float(block - start_block + 1) / blocks_total
            t_elapsed = time.time() - t_start
            t_togo = t_elapsed / progress

            if events is None:
                events = []

            print(f"scanning block {block} ({round(progress*100)}%) page : {page} - nr of events: {len(events)}, eta {t_togo/3600}h")

            for event in events:
                if event['module_id'] == 'encointerbalances':
                    noteworthy=False
                    if event['event_id'] == 'Transferred':
                        params = json.loads(event['params'])
                        cid = decode_cid(params[0]['value'])
                        account_from = ss58_encode(params[1]['value'], ss58_format=2)
                        account_to = ss58_encode(params[2]['value'], ss58_format=2)
                        amount = params[3]['value']
                        date = datetime.fromtimestamp(event['block_timestamp'])
                        noteworthy = True

                    if event['event_id'] == 'Issued':
                        params = json.loads(event['params'])
                        cid = decode_cid(params[0]['value'])
                        account_from = 'community-issued-income'
                        account_to = ss58_encode(params[1]['value'], ss58_format=2)
                        amount = params[2]['value']
                        date = datetime.fromtimestamp(event['block_timestamp'])
                        noteworthy = True

                    if noteworthy:
                        writer.writerow([
                            date,
                            cid,
                            account_from,
                            account_to,
                            str(amount),
                            event['event_index'],
                            event['extrinsic_hash']
                        ])
                        csvfile.flush()

            if len(events) < page_rows:
                break
            page += 1


