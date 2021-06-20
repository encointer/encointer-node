import sys

import flask
from flask import request, jsonify
import subprocess
from time import sleep
import binascii
import base58
from py_client.client import Client


app = flask.Flask(__name__)
app.config['DEBUG'] = True


CLI = ['../target/release/encointer-client-notee', '-p', '9944']
CLIENT = Client()
ACK_COMMUNITIES = []


def faucet(accounts):
    for x in range(0, 180):  # try 100 times
        try:
            ret = subprocess.run(CLI + ['faucet'] + accounts, check=True, timeout=2, stdout=subprocess.PIPE).stdout
            CLIENT.await_block()  # wait for transaction to complete
            for acc in accounts:
                if CLIENT.balance(acc) == 0:
                    break
            return True
        except subprocess.CalledProcessError as e:
            print(e.output)
        except subprocess.TimeoutExpired as e:
            print(e.output)
        sleep(1)
    print('failed')
    return False


def all_communities_ready(ack_communities):
    res = CLIENT.list_communities().splitlines()
    del res[0]
    all_communities = []
    for cid in res:
        all_communities.append(cid[15:81])
    print('ACK Communities')
    print(ACK_COMMUNITIES)
    print('All communities')
    print(all_communities)
    if set(ack_communities) == set(all_communities):
        return True
    return False


@app.route('/api', methods=['GET'])
def faucet_service():
    query_parameters = request.args
    accounts = query_parameters.getlist('accounts')
    res = faucet(accounts)
    return jsonify(success=res)


@app.route('/heartbeat', methods=['GET'])
def heartbeat():
    try:
        query_parameters = request.args
        cid = '0x' + binascii.hexlify(base58.b58decode(query_parameters.getlist('cid')[0])).decode('utf-8')
        print(cid)
        if cid not in ACK_COMMUNITIES:
            ACK_COMMUNITIES.append(cid)
        if all_communities_ready(ACK_COMMUNITIES) is True:
            CLIENT.next_phase()
            print('NEXT PHASE')
            ACK_COMMUNITIES.clear()
        return jsonify(success=True)
    except:
        print(sys.exc_info()[0])
        return jsonify(success=False)


app.run()

