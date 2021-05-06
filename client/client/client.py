import os
import subprocess


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

    def list_accounts(self):
        ret = subprocess.run(self.cli + ["list-accounts"], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").splitlines()

    def new_account(self):
        ret = subprocess.run(self.cli + ["new-account"], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def faucet(self, accounts):
        subprocess.run(self.cli + ["faucet"] + accounts, stdout=subprocess.PIPE)

    def balance(self, accounts, **kwargs):
        bal = []
        cid_arg = []
        if 'cid' in kwargs:
            cid_arg = ["--cid", kwargs.get('cid')]
        for account in accounts:
            ret = subprocess.run(self.cli + cid_arg + ["balance", account], stdout=subprocess.PIPE)
            bal.append(float(ret.stdout.strip().decode("utf-8").split(' ')[-1]))
        return bal

    def new_community(self, specfile, sender='//Alice'):
        ret = subprocess.run(self.cli + ["new-community", specfile, sender], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def await_block(self, amount=1):
        subprocess.run(self.cli + ["listen", "-b", str(amount)], stdout=subprocess.PIPE)

    def register_participant(self, account, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "register-participant", account], stdout=subprocess.PIPE)
        # print(ret.stdout.decode("utf-8"))

    def new_claim(self, account, vote, cid):
        ret = subprocess.run(self.cli + ["--cid", cid, "new-claim", account, str(vote)], stdout=subprocess.PIPE)
        return ret.stdout.decode("utf-8").strip()

    def sign_claim(self, account, claim):
        ret = subprocess.run(self.cli + ["sign-claim", account, claim], stdout=subprocess.PIPE)
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

    def register_attestations(self, account, attestations):
        ret = subprocess.run(self.cli + ["register-attestations", account] + attestations, stdout=subprocess.PIPE)
        # print(ret.stdout.decode("utf-8"))
