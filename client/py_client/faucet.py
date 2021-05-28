import flask
from flask import request, jsonify
import subprocess


app = flask.Flask(__name__)
app.config['DEBUG'] = True


CLI = ['../target/release/encointer-client-notee', '-p', '9944']

def faucet(accounts):
    return subprocess.run(CLI + ['faucet'] + accounts, stdout=subprocess.PIPE)


@app.route('/api', methods=['GET'])
def faucet_service():
    query_parameters = request.args
    accounts = query_parameters.getlist('accounts')
    results = faucet(accounts)
    return results


app.run()

