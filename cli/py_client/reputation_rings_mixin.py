from py_client.base import ensure_clean_exit


class _ReputationRingsMixin:
    def register_bandersnatch_key(self, account, key=None, pay_fees_in_cc=False):
        args = ["account", "bandersnatch-pubkey", "register", account]
        if key is not None:
            args += ["--key", key]
        ret = self.run_cli_command(args, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def initiate_rings(self, account, cid, ceremony_index, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["personhood", "ring", "initiate", account, "--ceremony-index", str(ceremony_index)],
            cid=cid, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def continue_ring_computation(self, account, pay_fees_in_cc=False):
        ret = self.run_cli_command(["personhood", "ring", "continue", account], pay_fees_in_cc=pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def get_rings(self, cid, ceremony_index):
        ret = self.run_cli_command(["personhood", "ring", "get", "--ceremony-index", str(ceremony_index)], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def prove_personhood(self, account, cid, ceremony_index, level=1, sub_ring=0, context="encointer-pop"):
        ret = self.run_cli_command([
            "personhood", "prove-ring-membership", account,
            "--ceremony-index", str(ceremony_index),
            "--level", str(level),
            "--sub-ring", str(sub_ring),
            "--context", context,
        ], cid=cid)
        ensure_clean_exit(ret)
        output = ret.stdout.decode("utf-8").strip()
        # Parse signature from output line "signature: 0x..."
        for line in output.split("\n"):
            if line.startswith("signature:"):
                return line.split(maxsplit=1)[1].strip(), output
        raise RuntimeError(f"prove-personhood did not return a signature: {output}")

    def verify_personhood(self, signature, cid, ceremony_index, level=1, sub_ring=0, context="encointer-pop"):
        ret = self.run_cli_command([
            "personhood", "verify-ring-membership",
            "--signature", signature,
            "--ceremony-index", str(ceremony_index),
            "--level", str(level),
            "--sub-ring", str(sub_ring),
            "--context", context,
        ], cid=cid)
        output = ret.stdout.decode("utf-8").strip()
        return "VALID" in output, output
