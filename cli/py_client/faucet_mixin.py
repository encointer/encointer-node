from py_client.base import ensure_clean_exit


class _FaucetMixin:
    def create_faucet(self, account, faucet_name, amount, drip_amount, whitelist, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["faucet", "create", account, faucet_name, str(amount), str(drip_amount)] + whitelist,
                                   cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def drip_faucet(self, account, faucet_account, cindex, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["faucet", "drip", account, faucet_account, str(cindex)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def dissolve_faucet(self, account, faucet_account, beneficiary, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["faucet", "dissolve", "--signer", account, faucet_account, beneficiary], cid,
                                   pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def close_faucet(self, account, faucet_account, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["faucet", "close", account, faucet_account], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def set_faucet_reserve_amount(self, account, amount, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["faucet", "set-reserve-amount", "--signer", account, str(amount)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def list_faucets(self, verbose=False):
        cmd = ["faucet", "list"]
        if verbose:
            cmd += ["-v"]
        ret = self.run_cli_command(cmd)
        return ret.stdout.decode("utf-8").strip()
