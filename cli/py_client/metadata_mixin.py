class _MetadataMixin:
    def print_metadata(self):
        ret = self.run_cli_command(["print-metadata"])
        return ret.stdout.decode("utf-8").strip()
