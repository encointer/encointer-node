from py_client.base import ensure_clean_exit


class _ReputationRingsMixin:
    def register_bandersnatch_key(self, account, key, pay_fees_in_cc=False):
        ret = self.run_cli_command(["register-bandersnatch-key", account, "--key", key],
                                   pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def initiate_rings(self, account, cid, ceremony_index, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["initiate-rings", account, "--ceremony-index", str(ceremony_index)],
            cid=cid, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def continue_ring_computation(self, account, pay_fees_in_cc=False):
        ret = self.run_cli_command(["continue-ring-computation", account], pay_fees_in_cc=pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def get_rings(self, cid, ceremony_index):
        ret = self.run_cli_command(["get-rings", "--ceremony-index", str(ceremony_index)], cid=cid)
        return ret.stdout.decode("utf-8").strip()
