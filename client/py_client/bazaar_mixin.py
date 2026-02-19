from py_client.base import ensure_clean_exit


class _BazaarMixin:
    def create_business(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["create-business", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def update_business(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["update-business", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def create_offering(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["create-offering", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def list_businesses(self, cid):
        ret = self.run_cli_command(["list-businesses"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_offerings(self, cid):
        ret = self.run_cli_command(["list-offerings"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_offerings_for_business(self, cid, account):
        ret = self.run_cli_command(["list-business-offerings", account], cid=cid)
        return ret.stdout.decode("utf-8").strip()
