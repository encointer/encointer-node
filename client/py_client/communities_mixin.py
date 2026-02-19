from py_client.base import ensure_clean_exit


class _CommunityMixin:
    def new_community(self, specfile, signer=None, wrap_call="none", batch_size=100, pay_fees_in_cc=False):
        cmd = ["community", "new", specfile]
        if signer:
            cmd += ["--signer", signer]

        cmd += ["--wrap-call", wrap_call, "--batch-size", str(batch_size)]
        ret = self.run_cli_command(cmd, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def list_communities(self):
        ret = self.run_cli_command(["community", "list"])
        return ret.stdout.decode("utf-8").strip()

    def add_locations(self, specfile, signer=None, cid=None, pay_fees_in_cc=False):
        cmd = ["community", "location", "add", specfile]
        if signer:
            cmd += ["--signer", signer]
        ret = self.run_cli_command(cmd, cid=cid, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def remove_location(self, signer, geohash, location_index=None, cid=None, pay_fees_in_cc=False):
        cmd = ["community", "location", "remove", "--signer", signer, "--geohash", geohash]
        if location_index is not None:
            cmd += ["--location-index", str(location_index)]
        ret = self.run_cli_command(cmd, cid=cid, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def list_locations(self, cid=None):
        ret = self.run_cli_command(["community", "location", "list"], cid=cid)
        return ret.stdout.decode("utf-8").strip()
