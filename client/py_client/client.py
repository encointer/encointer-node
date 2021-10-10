import subprocess
import requests

from py_client.scheduler import CeremonyPhase

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
                 rust_client="../target/release/encointer-client-notee",
                 port=9944
                 ):
        if node_url:
            print("node_url is true, node_url is:", node_url)
            self.cli = [rust_client, '-u', node_url, '-p', str(443)]
        else:
            print("node_url is false, node_url is:", node_url)
            self.cli = [rust_client, '-p', str(port)]

    def next_phase(self):
        ret = subprocess.run(self.cli + ["next-phase"])
        ensure_clean_exit(ret.returncode)

    def get_phase(self):
        ret = subprocess.run(self.cli + ["get-phase"], stdout=subprocess.PIPE)
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
        ret = subprocess.run(self.cli + ["list-accounts"], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").splitlines()

    def new_account(self):
        ret = subprocess.run(self.cli + ["new-account"], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def create_accounts(self, amount):
        return [self.new_account() for _ in range(0, amount)]

    def faucet(self, accounts, faucet_url='http://localhost:5000/api', is_faucet=False):
        if is_faucet:
            self.await_block(1)
            ret = subprocess.run(self.cli + ['faucet'] + accounts, check=True, timeout=2, stdout=subprocess.PIPE)
            print(ret.stdout.decode("utf-8"))
            ensure_clean_exit(ret.returncode)
        else:
            payload = {'accounts': accounts}
            requests.get(faucet_url, params=payload)


    def balance(self, account, cid=None):
        if not cid:
            ret = subprocess.run(self.cli + ["balance", account], stdout=subprocess.PIPE)
            return float(ret.stdout.strip().decode("utf-8").split(' ')[-1])
        else:
            ret = subprocess.run(self.cli + ["--cid", cid, "balance", account], stdout=subprocess.PIPE)
            return float(ret.stdout.strip().decode("utf-8").split(' ')[-1])

    def new_community(self, specfile, sender):
        ret = subprocess.run(self.cli + ["new-community", specfile, sender], stdout=subprocess.PIPE)
        ensure_clean_exit(ret.returncode)
        return ret.stdout.decode("utf-8").strip()

    def list_communities(self):
        ret = subprocess.run(self.cli + ["list-communities"], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def await_block(self, amount=1):
        subprocess.run(self.cli + ["listen", "-b", str(amount)], stdout=subprocess.PIPE)

    def list_participants(self, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "list-participants"], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def register_participant(self, account, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "register-participant", account], stdout=subprocess.PIPE)
        ensure_clean_exit(ret.returncode)

    def new_claim(self, account, vote, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "new-claim", account, str(vote)], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def list_meetups(self, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "list-meetups"], stdout=subprocess.PIPE)
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

    def attest_claims(self, account, claims):
        ret = subprocess.run(self.cli + ["attest-claims", account] + claims, stdout=subprocess.PIPE)
        ensure_clean_exit(ret.returncode)

    def list_attestees(self, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "list-attestees"], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def create_business(self, account, cid, ipfs_cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "create-business", account, "--ipfs-cid", ipfs_cid],
                             stdout=subprocess.PIPE)
        ensure_clean_exit(ret.returncode)
        return ret.stdout.decode("utf-8").strip()

    def update_business(self, account, cid, ipfs_cd):
        """ Update has not been tested """
        ret = subprocess.run(self.cli + ["--cid", cid, "update-business", account, "--ipfs-cid", ipfs_cd],
                             stdout=subprocess.PIPE)
        ensure_clean_exit(ret.returncode)
        return ret.stdout.decode("utf-8").strip()

    def create_offering(self, account, cid, ipfs_cd):
        ret = subprocess.run(self.cli + ["--cid", cid, "create-offering", account, "--ipfs-cid", ipfs_cd],
                             stdout=subprocess.PIPE)
        ensure_clean_exit(ret.returncode)
        return ret.stdout.decode("utf-8").strip()

    def list_businesses(self, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "list-businesses"],
                             stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def list_offerings(self, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "list-offerings"],
                             stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def list_offerings_for_business(self, cid, account):
        ret = subprocess.run(self.cli + ["--cid", cid, "list-business-offerings", account],
                             stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

