import time


SUPPRESSED_COMMANDS = {
    'export-secret', 'register-participant', 'register-offline-identity',
    'register-bandersnatch-key', 'reputation', 'attest-attendees',
    'claim-reward', 'new-account',
}


class SimulationLog:
    def __init__(self, path):
        self._file = open(path, 'w')
        self._start = time.monotonic()
        self.cindex = 0

    def _ts(self):
        elapsed = time.monotonic() - self._start
        m, s = divmod(int(elapsed), 60)
        return f"{m:02d}:{s:02d}|{self.cindex}"

    def command(self, args, returncode, stdout_snippet=''):
        if any(cmd in SUPPRESSED_COMMANDS for cmd in args):
            return
        status = "OK" if returncode == 0 else f"FAIL(rc={returncode})"
        line = f"  [{self._ts()}] {' '.join(args)}  → {status}"
        if stdout_snippet:
            line += f"  | {stdout_snippet}"
        self._file.write(line + '\n')

    def ceremony(self, cindex):
        self._file.write(f"\n{'═' * 60}\n CEREMONY {cindex}\n{'═' * 60}\n")

    def phase(self, name, cindex=None):
        tag = f" [{cindex}]" if cindex is not None else ""
        label = f"{name}{tag}"
        self._file.write(f"\n── {label} {'─' * max(1, 50 - len(label))}\n")

    def summary(self, text, cindex=None):
        tag = f" [{cindex}]" if cindex is not None else ""
        label = f"Summary{tag}"
        self._file.write(f"\n── {label} {'─' * max(1, 50 - len(label))}\n{text}\n")

    def close(self):
        self._file.flush()
        self._file.close()
