"""Utilities for monitoring the Substrate transaction pool."""
import time
from substrateinterface import SubstrateInterface


def create_substrate_connection(node_url='ws://127.0.0.1', port=9944):
    url = f"{node_url}:{port}"
    return SubstrateInterface(url=url, ss58_format=42)


def wait_for_pool_empty(substrate, timeout=60):
    """Poll author_pendingExtrinsics until the tx pool is empty."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        pending = substrate.rpc_request("author_pendingExtrinsics", [])
        if len(pending.get('result', [])) == 0:
            return
        time.sleep(1)
    raise TimeoutError(f"Tx pool not empty after {timeout}s")
