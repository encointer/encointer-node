import sys

import flask
from flask import request, jsonify
import subprocess
from time import sleep
from py_client.client import Client


app = flask.Flask(__name__)
app.config['DEBUG'] = True


CLI = ['../target/release/encointer-client-notee', '-p', '9944']
CLIENT = Client()


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


@app.route('/api', methods=['GET'])
def faucet_service():
    query_parameters = request.args
    accounts = query_parameters.getlist('accounts')
    res = faucet(accounts)
    return jsonify(success=res)


app.run()

