class _TreasuryMixin:
    def get_treasury(self, cid=None):
        ret = self.run_cli_command(["community", "get-treasury"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def get_swap_native_option(self, account, cid=None):
        ret = self.run_cli_command(["get-swap-native-option", account], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def get_swap_asset_option(self, account, cid=None):
        ret = self.run_cli_command(["get-swap-asset-option", account], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def swap_native(self, account, amount, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["swap-native", account, str(amount)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def swap_asset(self, account, amount, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["swap-asset", account, str(amount)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()
