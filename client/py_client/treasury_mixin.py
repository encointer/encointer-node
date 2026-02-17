class _TreasuryMixin:
    def get_treasury(self, cid=None):
        ret = self.run_cli_command(["get-treasury"], cid=cid)
        return ret.stdout.decode("utf-8").strip()
