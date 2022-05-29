#!/usr/bin/env python3
#
# ./fetch-account-history.py <account> <subscan api key>
#
import requests
import csv
import sys
import json
import time
from substrateinterface.utils.ss58 import ss58_decode, ss58_encode

if len(sys.argv) < 2:
    print("Usage: ./fetch.account-history.py <fund index> <subscan api key>")
    sys.exit()

account = sys.argv[1]
#api_key = sys.argv[2]

page_rows = 100

t_start = time.time()

start_block = 535620
end_block = 660815
blocks_total = end_block - start_block

with open(f'account-events-{account}.csv', 'w', newline='') as csvfile:
    writer = csv.writer(csvfile, delimiter=',')

    for block in range(start_block, end_block):
        page = 0
        while True:
            response = requests.post('https://encointer.api.subscan.io/api/scan/events',
                                     headers={
                                         'Content-Type': 'application/json',
                                         #'X-API-Key': api_key,
                                         'Accept': 'application/json',
                                     },
                                     json={
                                         'row': 100,
                                         'page': page,
                                         'module': 'encointerBalances',
                                         'block_num': block
                                     }
                                     )

            try:
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
            remaining = int(response.headers['X-RateLimit-Remaining-Second'])
            if remaining < 1:
                print(f"approaching rate limit {remaining}/{response.headers['X-RateLimit-Limit-Second']}")




            progress = float(block - start_block + 1) / blocks_total
            t_elapsed = time.time() - t_start
            t_togo = t_elapsed / progress
            print(f"scanning block {block}")

            if events is None:
                break

            print(f"scanning block {block} ({progress*100}%) page : {page} - nr of events: {len(events)}, eta {t_togo/3600}h")
   
            for event in events:
                if event['event_id'] == 'Transferred':
                    print(event)
                    params = json.loads(event['params'])
                    cid = params[0]['value']
                    account_from = ss58_encode(params[1]['value'], ss58_format=2)
                    account_to = ss58_encode(params[2]['value'], ss58_format=2) 
                    amount = params[3]['value']

                    writer.writerow([cid,
                                    account_from,
                                    account_to, 
                                    str(amount),
                                    str(block),
                                    ])
                    csvfile.flush()

            if len(events) < page_rows:
                break
            page += 1


