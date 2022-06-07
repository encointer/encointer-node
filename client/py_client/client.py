import subprocess
import requests
import os

from py_client.scheduler import CeremonyPhase

DEFAULT_CLIENT = '../target/release/encointer-client-notee'


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


def ensure_clean_exit(returncode):
    if returncode == 0:
        return
    if returncode == 50:
        raise ExtrinsicWrongPhase
    if returncode == 51:
        raise ExtrinsicFeePaymentImpossible
    if returncode == 52:
        raise ParticipantAlreadyLinked
    raise UnknownError


class Client:
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
            print("connecting to remote chain: ", node_url)
            self.cli = [rust_client, '-u', node_url, '-p', str(port)]
        else:
            print("connecting to local chain")
            self.cli = [rust_client, '-p', str(port)]

    def run_cli_command(self, command, cid=None, pay_fees_in_cc=False, ipfs_cid=None, **kwargs):
        cid_part = ["--cid", cid] if cid else []
        fee_part = ["--tx-payment-cid", cid] if pay_fees_in_cc else []
        ipfs_cid_part = ["--ipfs-cid", ipfs_cid] if ipfs_cid else []
        command = self.cli + cid_part + fee_part + ipfs_cid_part + command
        ret = subprocess.run(command, stdout=subprocess.PIPE, **kwargs)
        return ret

    def next_phase(self):
        ret = self.run_cli_command(["next-phase"])
        ensure_clean_exit(ret.returncode)

    def get_phase(self):
        ret = self.run_cli_command(["get-phase"])
        return ret.stdout.strip().decode("utf-8")

    def go_to_phase(self, phase):
        print("Advancing to phase: " + str(phase))
        while True:
            p = CeremonyPhase[self.get_phase()]
            if p == phase:
                print(f"Arrived at {p}.")
                return
            else:
                print(f"Phase is: {p}. Need to advance")
                self.next_phase()

    def list_accounts(self):
        ret = self.run_cli_command(["list-accounts"])
        return ret.stdout.decode("utf-8").splitlines()

    def new_account(self):
        ret = self.run_cli_command(["new-account"])
        return ret.stdout.decode("utf-8").strip()

    def create_accounts(self, amount):
        return [self.new_account() for _ in range(0, amount)]

    def faucet(self, accounts, faucet_url='http://localhost:5000/api', is_faucet=False, pay_fees_in_cc=False):
        if is_faucet:
            self.await_block(1)
            ret = self.run_cli_command(
                ['faucet'] + accounts, pay_fees_in_cc=pay_fees_in_cc, check=True, timeout=2)
            print(ret.stdout.decode("utf-8"))
            ensure_clean_exit(ret.returncode)
        else:
            payload = {'accounts': accounts}
            requests.get(faucet_url, params=payload)

    def balance(self, account, cid=None):
        ret = self.run_cli_command(["balance", account], cid=cid)
        return float(ret.stdout.strip().decode("utf-8").split(' ')[-1])

    def reputation(self, account):
        ret = self.run_cli_command(["reputation", account])
        ensure_clean_exit(ret.returncode)
        reputation_history = []
        lines = ret.stdout.decode("utf-8").splitlines()
        while len(lines) > 0:
            (cindex, cid, rep) = lines.pop(0).split(',')
            reputation_history.append(
                (cindex, cid, rep.strip().split('::')[1]))
        return reputation_history

    def new_community(self, specfile):
        ret = self.run_cli_command(["new-community", specfile])
        ensure_clean_exit(ret.returncode)
        return ret.stdout.decode("utf-8").strip()

    def list_communities(self):
        ret = self.run_cli_command(["list-communities"])
        return ret.stdout.decode("utf-8").strip()

    def await_block(self, amount=1):
        self.run_cli_command(["listen", "-b", str(amount)])

    def list_participants(self, cid):
        ret = self.run_cli_command(["list-participants"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def register_participant(self, account, cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["register-participant", account], cid, pay_fees_in_cc)
        ensure_clean_exit(ret.returncode)

    def new_claim(self, account, vote, cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["new-claim", account, str(vote)], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_meetups(self, cid):
        ret = self.run_cli_command(["list-meetups"], cid)
        # print(ret.stdout.decode("utf-8"))
        meetups = []
        lines = ret.stdout.decode("utf-8").splitlines()
        while len(lines) > 0:
            if 'participants are:' in lines.pop(0):
                participants = []
                while len(lines) > 0:
                    l = lines.pop(0)
                    if 'MeetupRegistry' in l:
                        break
                    participants.append(l.strip())
                meetups.append(participants)
        return meetups

    def attest_claims(self, account, claims, pay_fees_in_cc=False):
        ret = self.run_cli_command(["attest-claims", account] +
                                   claims, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret.returncode)

    def list_attestees(self, cid):
        ret = self.run_cli_command(["list-attestees"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def claim_reward(self, account, cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["claim-reward", account], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def create_business(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["create-business", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret.returncode)
        return ret.stdout.decode("utf-8").strip()

    def update_business(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        """ Update has not been tested """
        ret = self.run_cli_command(
            ["update-business", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret.returncode)
        return ret.stdout.decode("utf-8").strip()

    def create_offering(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["create-offering", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret.returncode)
        return ret.stdout.decode("utf-8").strip()

    def list_businesses(self, cid):
        ret = self.run_cli_command(["list-businesses"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_offerings(self, cid):
        ret = self.run_cli_command(["list-offerings"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_offerings_for_business(self, cid, account):
        ret = self.run_cli_command(
            ["list-business-offerings", account], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def endorse_newcomers(self, cid, endorser, endorsees, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["endorse-newcomers", endorser, "--endorsees"] +
            endorsees,  # must be separate to append a list of args to the cli
            cid,
            pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def get_bootstrappers_with_remaining_newbie_tickets(self, cid):
        ret = self.run_cli_command(
            ["get-bootstrappers-with-remaining-newbie-tickets"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def transfer_all(self, cid, source, dest, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["transfer_all", source, dest], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()
