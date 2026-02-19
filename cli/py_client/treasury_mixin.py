class _TreasuryMixin:
    def get_treasury(self, cid=None):
        ret = self.run_cli_command(["community", "treasury", "get-account"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def get_swap_native_option(self, account, cid=None):
        ret = self.run_cli_command(["community", "treasury", "swap-option", "get-native", account], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def get_swap_asset_option(self, account, cid=None):
        ret = self.run_cli_command(["community", "treasury", "swap-option", "get-asset", account], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def swap_native(self, account, amount, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["community", "treasury", "swap-option", "exercise-native", account, str(amount)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def swap_asset(self, account, amount, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["community", "treasury", "swap-option", "exercise-asset", account, str(amount)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()
