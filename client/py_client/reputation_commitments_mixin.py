class _ReputationCommitmentsMixin:
    def list_commitments(self, purpose_id=None):
        cmd = ["reputation", "list-commitments"]
        if purpose_id is not None:
            cmd += ["--purpose-id", str(purpose_id)]
        ret = self.run_cli_command(cmd)
        return ret.stdout.decode("utf-8").strip()

    def list_purposes(self):
        ret = self.run_cli_command(["reputation", "list-purposes"])
        return ret.stdout.decode("utf-8").strip()
