from py_client.campaign import Campaign


class SwapOptionCampaign(Campaign):
    """Two merchants submit swap-native-option proposals, community votes,
    then merchants exercise their options after enactment.

    Timeline:
      cindex 5 (on_post_ceremony): fund treasury, submit proposals
      cindex 6 (on_attesting):     vote on proposals
      cindex 6 (on_post_ceremony): update proposal states
      cindex 7 (on_post_ceremony): check enactment, query & exercise options
    """

    SUBMIT_CINDEX = 5
    VOTE_CINDEX = 6
    EXERCISE_CINDEX = 7
    NATIVE_ALLOWANCE = 1_000_000_000_000  # 1 KSM in pico
    RATE = 100_000  # CC per native token
    TREASURY_FUND = 10_000_000_000_000  # 10 KSM in pico

    def __init__(self, pool, log=None):
        super().__init__(pool, log)
        self._proposal_ids = []
        self._merchants = []

    def on_post_ceremony(self, cindex):
        if cindex == self.SUBMIT_CINDEX:
            self._submit_proposals()
        elif cindex == self.VOTE_CINDEX:
            self._update_and_check()
        elif cindex == self.EXERCISE_CINDEX:
            self._exercise_options()

    def on_attesting(self, cindex):
        if cindex == self.VOTE_CINDEX:
            self._vote_aye()

    def _submit_proposals(self):
        """Fund community treasury and submit swap-native-option proposals."""
        merchants = [a for a in self.pool.agents if a.has_business][:2]
        if len(merchants) < 2:
            print("  Campaign swap_option: not enough merchants, skipping")
            return
        self._merchants = merchants

        # Fund the community treasury with native tokens
        treasury = self.client.get_treasury(cid=self.cid)
        print(f"🏦 Campaign swap_option: treasury account = {treasury}")

        # Transfer native tokens from Alice to treasury
        print(f"  funding treasury with {self.TREASURY_FUND} native tokens")
        self.client.transfer(None, "//Alice", treasury, str(self.TREASURY_FUND))
        self.pool._wait()

        native_bal = self.client.balance(treasury)
        print(f"  treasury native balance: {native_bal}")

        # Each merchant submits a proposal
        proposer = self.pool._first_reputable()
        for i, merchant in enumerate(self._merchants):
            print(f"  merchant {i}: {merchant.account[:8]}... submitting swap-native-option proposal")
            self.client.submit_issue_swap_native_option_proposal(
                account=proposer.account,
                to=merchant.account,
                native_allowance=self.NATIVE_ALLOWANCE,
                rate=self.RATE,
                do_burn=False,
                cid=self.cid,
            )
            self.pool._wait()

        # Find our proposal IDs
        proposals = self.client.get_proposals()
        self._proposal_ids = [
            p.id for p in proposals
            if 'SwapNativeOption' in p.action and p.state == 'Ongoing'
        ]
        print(f"  submitted {len(self._proposal_ids)} swap-native-option proposals: {self._proposal_ids}")

    def _vote_aye(self):
        """All reputables vote aye on swap-native-option proposals."""
        if not self._proposal_ids:
            return

        proposals = self.client.get_proposals()
        swap_proposals = [p for p in proposals if p.id in self._proposal_ids and p.state == 'Ongoing']
        if not swap_proposals:
            print("  Campaign swap_option: no ongoing swap proposals to vote on")
            return

        print(f"🗳 Campaign swap_option: voting aye on {len(swap_proposals)} proposals")
        voters = [a for a in self.pool.agents if a.is_reputable]
        for proposal in swap_proposals:
            voted = 0
            for voter in voters:
                reputations = [[t[1], t[0]] for t in self.client.reputation(voter.account)]
                if not reputations:
                    continue
                try:
                    self.client.vote(voter.account, proposal.id, 'aye', reputations)
                    voted += 1
                except Exception as e:
                    pass  # some may have already voted via base democracy
            print(f"  proposal {proposal.id}: {voted} aye votes cast")
            self.pool._wait()

    def _update_and_check(self):
        """Update proposal states and check for approval."""
        if not self._proposal_ids:
            return

        print("📋 Campaign swap_option: updating proposal states")
        updater = self.pool.agents[0].account
        for pid in self._proposal_ids:
            self.client.update_proposal_state(updater, pid)
        self.pool._wait()

        proposals = self.client.get_proposals()
        for p in proposals:
            if p.id in self._proposal_ids:
                print(f"  proposal {p.id}: state={p.state}, turnout={p.turnout}, approval={p.approval}")

    def _exercise_options(self):
        """After enactment: query swap options and exercise them partially."""
        if not self._merchants:
            return

        # Final state update to trigger enactment
        print("💱 Campaign swap_option: checking enactment and exercising options")
        updater = self.pool.agents[0].account
        for pid in self._proposal_ids:
            try:
                self.client.update_proposal_state(updater, pid)
            except Exception:
                pass
        self.pool._wait()

        # Check final proposal states
        proposals = self.client.get_proposals()
        enacted = [p for p in proposals if p.id in self._proposal_ids and p.state == 'Enacted']
        print(f"  enacted proposals: {len(enacted)} / {len(self._proposal_ids)}")

        for merchant in self._merchants:
            # Query the swap option
            option_str = self.client.get_swap_native_option(merchant.account, cid=self.cid)
            print(f"  {merchant.account[:8]}... option: {option_str[:120]}")

            if "No swap" in option_str:
                print(f"    no option found, skipping exercise")
                continue

            # Exercise half the allowance
            exercise_amount = self.NATIVE_ALLOWANCE // 2
            print(f"    exercising {exercise_amount} of {self.NATIVE_ALLOWANCE}")
            try:
                result = self.client.swap_native(merchant.account, exercise_amount, cid=self.cid)
                print(f"    swap result: {result}")
            except Exception as e:
                print(f"    swap failed: {e}")

            self.pool._wait()

            # Query remaining option
            remaining = self.client.get_swap_native_option(merchant.account, cid=self.cid)
            print(f"    remaining option: {remaining[:120]}")

    def write_summary(self, cindex):
        if self.log is None:
            return
        if cindex == self.EXERCISE_CINDEX and self._merchants:
            self.log.phase('Campaign: swap_option')
            self.log._file.write(f"  Merchants: {len(self._merchants)}\n")
            self.log._file.write(f"  Proposals: {len(self._proposal_ids)}\n")
            proposals = self.client.get_proposals()
            enacted = sum(1 for p in proposals if p.id in self._proposal_ids and p.state == 'Enacted')
            self.log._file.write(f"  Enacted: {enacted}\n")
