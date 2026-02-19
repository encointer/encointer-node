import requests


class _BalanceMixin:
    def faucet(self, accounts, faucet_url='http://localhost:5000/api', is_faucet=False, pay_fees_in_cc=False):
        if is_faucet:
            print(f"we are the faucet")
            self.await_block(1)
            from py_client.base import ensure_clean_exit
            ret = self.run_cli_command(
                ['account', 'fund'] + accounts, pay_fees_in_cc=pay_fees_in_cc, check=True, timeout=2)
            print(ret.stdout.decode("utf-8"))
            ensure_clean_exit(ret)
        else:
            print(f"connecting to faucet: {faucet_url}")
            payload = {'accounts': accounts}
            try:
                requests.get(faucet_url, params=payload, timeout=20)
            except requests.exceptions.Timeout:
                print("faucet timeout")

    def balance(self, account, cid=None):
        ret = self.run_cli_command(["balance", account], cid=cid)
        return float(ret.stdout.strip().decode("utf-8").split(' ')[-1])

    def transfer_all(self, cid, source, dest, pay_fees_in_cc=False):
        ret = self.run_cli_command(["transfer-all", source, dest], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def transfer(self, cid, source, dest, amount, pay_fees_in_cc=False):
        ret = self.run_cli_command(["transfer", source, dest, amount], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def issuance(self, cid):
        ret = self.run_cli_command(["community", "issuance"], cid=cid)
        return ret.stdout.decode("utf-8").strip()
