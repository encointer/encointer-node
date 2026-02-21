import subprocess
import os
import time

from py_client.scheduler import CeremonyPhase

DEFAULT_CLIENT = '../target/release/encointer-cli'


class Error(Exception):
    """Base class for exceptions in this module."""
    pass


class ExtrinsicWrongPhase(Error):
    """"it is not the right ceremony phase for this extrinsic"""
    pass


class ExtrinsicFeePaymentImpossible(Error):
    """Signer can't pay fees. Either because account does not exist or the balance is too low"""
    pass


class ParticipantAlreadyLinked(Error):
    """Can't register participant. reputation has already been linked"""
    pass


class UnknownError(Error):
    pass


def ensure_clean_exit(ret):
    returncode = ret.returncode
    if returncode == 0:
        return
    print(ret)
    if returncode == 50:
        raise ExtrinsicWrongPhase
    if returncode == 51:
        raise ExtrinsicFeePaymentImpossible
    if returncode == 52:
        raise ParticipantAlreadyLinked
    raise UnknownError


class _BaseClient:
    def __init__(self,
                 node_url=None,
                 rust_client=None,
                 port=9944
                 ):
        if not rust_client:
            try:
                rust_client = os.environ['ENCOINTER_CLIENT']
            except:
                print(
                    f"didn't find ENCOINTER_CLIENT in env variables nor arguments, setting client to {DEFAULT_CLIENT}")
                rust_client = DEFAULT_CLIENT

        if node_url:
            print("🔌 connecting to remote chain: ", node_url)
            self.cli = [rust_client, '-u', node_url, '-p', str(port)]
        else:
            print("🔌 connecting to local chain")
            self.cli = [rust_client, '-p', str(port)]

        self.log = None

    def run_cli_command(self, command, cid=None, pay_fees_in_cc=False, ipfs_cid=None, **kwargs):
        cid_part = ["--cid", cid] if cid else []
        fee_part = ["--tx-payment-cid", cid] if pay_fees_in_cc else []
        ipfs_cid_part = ["--ipfs-cid", ipfs_cid] if ipfs_cid else []
        full_command = self.cli + cid_part + fee_part + command + ipfs_cid_part
        ret = subprocess.run(full_command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)
        if self.log is not None:
            meaningful = cid_part + fee_part + command + ipfs_cid_part
            stdout_first = ret.stdout.decode('utf-8', errors='replace').strip().split('\n')[0][:120]
            self.log.command(meaningful, ret.returncode, stdout_first)
        return ret

    def next_phase(self, pay_fees_in_cc=False):
        ret = self.run_cli_command(["ceremony", "admin", "next-phase"], pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)

    def get_phase(self):
        ret = self.run_cli_command(["ceremony", "phase"])
        return ret.stdout.strip().decode("utf-8")

    def get_cindex(self):
        ret = self.run_cli_command(["ceremony", "index"])
        return int(ret.stdout.strip().decode("utf-8"))

    def go_to_phase(self, phase, timeout=120):
        """Advance to target phase via polling. Resilient to concurrent phase.py."""
        print("⏱ Advancing to phase: " + str(phase))
        deadline = time.monotonic() + timeout
        while True:
            current = CeremonyPhase[self.get_phase()]
            if current == phase:
                print(f"⏱ Arrived at {current}.")
                return
            if time.monotonic() > deadline:
                raise TimeoutError(f"Timed out reaching {phase} (stuck at {current})")
            print(f"⏱ Phase is: {current}. Need to advance")
            self.next_phase()
            while CeremonyPhase[self.get_phase()] == current:
                if time.monotonic() > deadline:
                    raise TimeoutError(f"Timed out reaching {phase} (stuck at {current})")
                time.sleep(1)

    def list_accounts(self):
        ret = self.run_cli_command(["account", "list"])
        return ret.stdout.decode("utf-8").splitlines()

    def new_account(self):
        ret = self.run_cli_command(["account", "new"])
        return ret.stdout.decode("utf-8").strip()

    def export_secret(self, account):
        ret = self.run_cli_command(["account", "export", account])
        return ret.stdout.decode("utf-8").strip().strip('"')

    def create_accounts(self, amount):
        return [self.new_account() for _ in range(0, amount)]

    def await_block(self, amount=1):
        self.run_cli_command(["listen", "-b", str(amount)])
