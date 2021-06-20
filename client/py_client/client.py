import subprocess
import requests

from py_client.scheduler import CeremonyPhase


class Client:
    def __init__(self,
                 rust_client="../target/release/encointer-client-notee",
                 port=9944
                 ):
        self.cli = [rust_client, '-p', str(port)]

    def next_phase(self):
        subprocess.run(self.cli + ["next-phase"])

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

    def faucet(self, accounts, faucet_url='http://localhost:5000/api'):
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
        # print(ret.stdout.decode("utf-8"))

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
        # print(ret.stdout.decode("utf-8"))

    def list_attestees(self, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "list-attestees"], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def send_heartbeat(self, cid, heartbeat_url='http://localhost:5000/api'):
        payload = {'cid': cid}
        requests.get(heartbeat_url, params=payload)
