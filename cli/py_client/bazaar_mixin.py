from py_client.base import ensure_clean_exit


class _BazaarMixin:
    def create_business(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["bazaar", "business", "create", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def update_business(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["bazaar", "business", "update", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def create_offering(self, account, cid, ipfs_cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["bazaar", "offering", "create", account], cid, pay_fees_in_cc, ipfs_cid)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def list_businesses(self, cid):
        ret = self.run_cli_command(["bazaar", "business", "list"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_offerings(self, cid):
        ret = self.run_cli_command(["bazaar", "offering", "list"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_offerings_for_business(self, cid, account):
        ret = self.run_cli_command(["bazaar", "business", "offerings", account], cid=cid)
        return ret.stdout.decode("utf-8").strip()
