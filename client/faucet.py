import flask
from flask import request, jsonify
import subprocess
from time import sleep
from py_client.client import Client


app = flask.Flask(__name__)
app.config['DEBUG'] = True


CLI = ['../target/release/encointer-client-notee', '-p', '9944']


def faucet(accounts, client=Client()): 
    for x in range(0, 180):  # try 10 times
        print(x)
        try:
            subprocess.check_output(CLI + ['faucet'] + accounts, timeout=2)  # call faucet
            client.await_block()  # wait for transaction to complete
            bal = client.balance(accounts[0])
            if bal > 0:  # check if transaction was successful
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

