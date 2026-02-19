class Campaign:
    """Base class for simulation campaigns with no-op hooks."""

    def __init__(self, pool, log=None):
        self.pool = pool
        self.client = pool.client
        self.cid = pool.cid
        self.log = log

    def on_registering(self, cindex):
        pass

    def on_assigning(self, cindex):
        pass

    def on_attesting(self, cindex):
        pass

    def on_post_ceremony(self, cindex):
        pass

    def write_summary(self, cindex):
        pass
