class _IpfsMixin:
    def ipfs_upload(self, signer, file_path, cid, gateway_url=None):
        """Upload file to IPFS via authenticated gateway.
        Returns (success: bool, output: str, exit_code: int)
        """
        cmd = ["ipfs", "upload", "--signer", signer]
        if gateway_url:
            cmd += ["--gateway", gateway_url]
        cmd += [file_path]
        ret = self.run_cli_command(cmd, cid=cid)
        output = ret.stdout.decode("utf-8").strip()
        if ret.stderr:
            stderr_text = ret.stderr.decode("utf-8").strip()
            if stderr_text:
                output = f"{output}\n{stderr_text}".strip()
        return (ret.returncode == 0, output, ret.returncode)
