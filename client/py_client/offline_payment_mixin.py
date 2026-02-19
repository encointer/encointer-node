from py_client.base import ensure_clean_exit


class _OfflinePaymentMixin:
    def register_offline_identity(self, account, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["account", "poseidon-commitment", "register", account], cid=cid, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def get_offline_identity(self, account, cid=None):
        ret = self.run_cli_command(["account", "poseidon-commitment", "get", account], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def generate_offline_payment(self, signer, to, amount, cid=None, pk_file=None):
        cmd = ["offline-payment", "pay", "--signer", signer, "--to", to, "--amount", str(amount)]
        if pk_file:
            cmd += ["--pk-file", pk_file]
        ret = self.run_cli_command(cmd, cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def submit_offline_payment(self, signer, proof_file=None, proof=None, sender=None, recipient=None,
                               amount=None, nullifier=None, cid=None, pay_fees_in_cc=False):
        cmd = ["offline-payment", "settle", "--signer", signer]
        if proof_file:
            cmd += ["--proof-file", proof_file]
        if proof:
            cmd += ["--proof", proof]
        if sender:
            cmd += ["--sender", sender]
        if recipient:
            cmd += ["--recipient", recipient]
        if amount is not None:
            cmd += ["--amount", str(amount)]
        if nullifier:
            cmd += ["--nullifier", nullifier]
        ret = self.run_cli_command(cmd, cid=cid, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def set_offline_payment_vk(self, signer="//Alice", vk_file=None, vk=None, pay_fees_in_cc=False):
        cmd = ["offline-payment", "admin", "set-vk", "--signer", signer]
        if vk_file:
            cmd += ["--vk-file", vk_file]
        if vk:
            cmd += ["--vk", vk]
        ret = self.run_cli_command(cmd, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()

    def generate_test_vk(self):
        ret = self.run_cli_command(["offline-payment", "admin", "generate-test-vk"])
        return ret.stdout.decode("utf-8").strip()

    def generate_trusted_setup(self, pk_out="proving_key.bin", vk_out="verifying_key.bin"):
        ret = self.run_cli_command([
            "offline-payment", "admin", "trusted-setup", "generate", "--pk-out", pk_out, "--vk-out", vk_out])
        return ret.stdout.decode("utf-8").strip()

    def verify_trusted_setup(self, pk, vk):
        ret = self.run_cli_command(["offline-payment", "admin", "trusted-setup", "verify", "--pk", pk, "--vk", vk])
        return ret.stdout.decode("utf-8").strip()

    def ceremony_init(self, pk_out="ceremony_pk.bin", transcript="ceremony_transcript.json"):
        ret = self.run_cli_command([
            "offline-payment", "admin", "ceremony", "init", "--pk-out", pk_out, "--transcript", transcript])
        return ret.stdout.decode("utf-8").strip()

    def ceremony_contribute(self, participant, pk="ceremony_pk.bin", transcript="ceremony_transcript.json"):
        ret = self.run_cli_command([
            "offline-payment", "admin", "ceremony", "contribute", "--pk", pk, "--transcript", transcript, "--participant", participant])
        return ret.stdout.decode("utf-8").strip()

    def ceremony_verify(self, pk="ceremony_pk.bin", transcript="ceremony_transcript.json"):
        ret = self.run_cli_command(["offline-payment", "admin", "ceremony", "verify", "--pk", pk, "--transcript", transcript])
        return ret.stdout.decode("utf-8").strip()

    def ceremony_finalize(self, pk="ceremony_pk.bin", pk_out="proving_key.bin", vk_out="verifying_key.bin"):
        ret = self.run_cli_command([
            "offline-payment", "admin", "ceremony", "finalize", "--pk", pk, "--pk-out", pk_out, "--vk-out", vk_out])
        return ret.stdout.decode("utf-8").strip()

    def inspect_setup_key(self, file_path):
        ret = self.run_cli_command(["offline-payment", "admin", "inspect-key", "--file", file_path])
        return ret.stdout.decode("utf-8").strip()
