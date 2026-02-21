#!/usr/bin/env python3
"""
Combined ceremony phase controller and faucet HTTP service.

Phase coordination:
  Communities register via /register?cid=XXX, signal readiness via /ready?cid=XXX.
  Phase advances only when ALL registered communities are ready and the tx pool is empty.

Faucet:
  Fund accounts via /api?accounts=ADDR1&accounts=ADDR2

All //Alice operations (faucet drip, phase advance) are serialized via a lock,
eliminating nonce clashes.

Usage:
  python ceremony-phase-and-faucet-service.py --service-port 7070

Bot-community integration:
  curl http://localhost:7070/register?cid=MyCid
  curl http://localhost:7070/ready?cid=MyCid
  curl http://localhost:7070/unregister?cid=MyCid
  curl "http://localhost:7070/api?accounts=5GrwvaEF..."
"""

import time
import threading
import json
import urllib.request
import click
from flask import Flask, request as flask_request, jsonify
from py_client.helpers import set_local_or_remote_chain

app = Flask(__name__)

_condition = threading.Condition()
_registered = set()
_ready = set()
_generation = 0
_client = None
_rpc_url = None
_alice_lock = threading.Lock()


# ── Faucet ──────────────────────────────────────────────────────────────────

@app.route('/api', methods=['GET'])
def faucet_service():
    accounts = flask_request.args.getlist('accounts')
    print(f'faucet: got request to fund {accounts}')
    if len(accounts) == 0:
        return "no accounts provided to drip to\n"
    with _alice_lock:
        _client.faucet(accounts, is_faucet=True)
        _client.await_block()
    return jsonify(success=True)


# ── Phase coordination ──────────────────────────────────────────────────────

@app.route('/register')
def register():
    cid = flask_request.args.get('cid')
    if not cid:
        return "missing cid parameter\n", 400
    with _condition:
        _registered.add(cid)
        _ready.discard(cid)
    print(f'registered {cid} ({len(_registered)} total)')
    return f"registered {cid}\n"


@app.route('/unregister')
def unregister():
    global _generation
    cid = flask_request.args.get('cid')
    if not cid:
        return "missing cid parameter\n", 400
    with _condition:
        _registered.discard(cid)
        _ready.discard(cid)
        if len(_registered) > 0 and _ready >= _registered:
            phase = _advance()
            _generation += 1
            _ready.clear()
            _condition.notify_all()
    print(f'unregistered {cid} ({len(_registered)} total)')
    return f"unregistered {cid}\n"


@app.route('/ready')
def mark_ready():
    global _generation
    cid = flask_request.args.get('cid')
    if not cid:
        return "missing cid parameter\n", 400
    with _condition:
        if cid not in _registered:
            return f"community {cid} not registered\n", 400
        _ready.add(cid)
        my_gen = _generation
        print(f'{cid} ready ({len(_ready)}/{len(_registered)})')
        if _ready >= _registered:
            phase = _advance()
            _generation += 1
            _ready.clear()
            _condition.notify_all()
            return f"{phase}\n"
        else:
            while _generation == my_gen:
                _condition.wait()
            return f"{_client.get_phase()}\n"


# ── Internal helpers ────────────────────────────────────────────────────────

def _pending_count():
    """Check pending extrinsics via stateless HTTP JSON-RPC."""
    payload = json.dumps({"id": 1, "jsonrpc": "2.0", "method": "author_pendingExtrinsics", "params": []}).encode()
    req = urllib.request.Request(_rpc_url, data=payload, headers={"Content-Type": "application/json"})
    resp = urllib.request.urlopen(req)
    return len(json.loads(resp.read()).get('result', []))


def _wait_pool_empty(timeout=30):
    """Wait for tx pool to drain (register/attest txs included in blocks)."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if _pending_count() == 0:
            return
        time.sleep(1)
    print(f"WARNING: pool not empty after {timeout}s, advancing anyway")


def _advance():
    """Wait for pool to drain, then advance the ceremony phase under Alice lock."""
    _wait_pool_empty()
    with _alice_lock:
        for attempt in range(5):
            try:
                _client.next_phase()
                phase = _client.get_phase()
                print(f'NEXT PHASE! → {phase}')
                return phase
            except Exception as e:
                print(f'next_phase attempt {attempt + 1} failed: {e}, retrying in 6s...')
                time.sleep(6)
    raise RuntimeError("Failed to advance phase after 5 attempts")


def _get_node_url(node_url, port):
    if node_url == "gesell":
        return f"wss://gesell.encointer.org:{443}"
    else:
        return f"{node_url}:{port}"


# ── Main ────────────────────────────────────────────────────────────────────

@click.command()
@click.option('--client', default='../target/release/encointer-cli',
              help='Client binary to communicate with the chain.')
@click.option('-u', '--url', default='ws://127.0.0.1', help='URL of the chain, or `gesell` alternatively.')
@click.option('-p', '--port', default='9944', help='ws-port of the chain.')
@click.option('--service-port', default=7070, help='HTTP service port')
def main(client, url, port, service_port):
    global _client, _rpc_url
    _client = set_local_or_remote_chain(client, port, url)
    node_url = _get_node_url(node_url=url, port=port)
    _rpc_url = node_url.replace('ws://', 'http://').replace('wss://', 'https://')
    print(f'Ceremony phase and faucet service listening on :{service_port}')
    app.run(host='0.0.0.0', port=service_port, threaded=True)


if __name__ == '__main__':
    main()
