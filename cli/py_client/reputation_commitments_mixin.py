class _ReputationCommitmentsMixin:
    def list_commitments(self, purpose_id=None):
        cmd = ["personhood", "commitment", "list"]
        if purpose_id is not None:
            cmd += ["--purpose-id", str(purpose_id)]
        ret = self.run_cli_command(cmd)
        return ret.stdout.decode("utf-8").strip()

    def list_purposes(self):
        ret = self.run_cli_command(["personhood", "commitment", "purposes"])
        return ret.stdout.decode("utf-8").strip()
