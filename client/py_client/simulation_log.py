import time


class SimulationLog:
    def __init__(self, path):
        self._file = open(path, 'w')
        self._start = time.monotonic()

    def _ts(self):
        elapsed = time.monotonic() - self._start
        m, s = divmod(int(elapsed), 60)
        return f"{m:02d}:{s:02d}"

    def command(self, args, returncode, stdout_snippet=''):
        status = "OK" if returncode == 0 else f"FAIL(rc={returncode})"
        line = f"  [{self._ts()}] {' '.join(args)}  → {status}"
        if stdout_snippet:
            line += f"  | {stdout_snippet}"
        self._file.write(line + '\n')

    def ceremony(self, cindex):
        self._file.write(f"\n{'═' * 60}\n CEREMONY {cindex}\n{'═' * 60}\n")

    def phase(self, name):
        self._file.write(f"\n── {name} {'─' * (50 - len(name))}\n")

    def summary(self, text):
        self._file.write(f"\n── Summary {'─' * 42}\n{text}\n")

    def close(self):
        self._file.flush()
        self._file.close()
