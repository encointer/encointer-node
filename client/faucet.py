#!/usr/bin/env python3
"""
start a http service that acts as a faucet.

test with
curl -X GET http://localhost:5000/api?accounts=5GpStrTKCVJNfhF8qUrQaKCQxHLnpVXmjM1nmw9LLv3rZYRF

"""

import sys
import flask
from flask import request, jsonify
import subprocess
from time import sleep
import click

from py_client.client import Client

app = flask.Flask(__name__)
app.config['DEBUG'] = True
CLIENT = None


def faucet(accounts):
    for x in range(0, 1):  # try multiple
        try:
            CLIENT.faucet(accounts, is_faucet=True)
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


@app.route('/api', methods=['GET'])
def faucet_service():
    query_parameters = request.args
    print(f'request args {query_parameters}')
    accounts = query_parameters.getlist('accounts')
    print(f'got request to fund {accounts}')
    if len(accounts) > 0:
        res = faucet(accounts)
        return jsonify(success=res)
    else:
        return "no accounts provided to drip to\n"


@click.command()
@click.option('--client', default='../target/release/encointer-client-notee',
              help='Client binary to communicate with the chain.')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain.')
@click.option('-p', '--port', default='9944', help='ws-port of the chain.')
def main(client, url, port):
    CLIENT = Client(rust_client=client, node_url=url, port=port)
    app.run()


if __name__ == "__main__":
    main()
