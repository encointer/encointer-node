from concurrent.futures import ThreadPoolExecutor, as_completed

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

    def on_registering(self, cindex):
        if cindex > self.SUBMIT_CINDEX:
            self._maybe_update_proposals()

    def on_assigning(self, cindex):
        if cindex > self.SUBMIT_CINDEX:
            self._maybe_update_proposals()

    def on_attesting(self, cindex):
        if cindex > self.SUBMIT_CINDEX:
            self._maybe_update_proposals()
        if cindex == self.VOTE_CINDEX:
            self._vote_aye()

    def on_post_ceremony(self, cindex):
        try:
            if cindex == self.SUBMIT_CINDEX:
                self._submit_proposals()
            elif cindex == self.VOTE_CINDEX:
                self._update_and_check()
            elif cindex == self.EXERCISE_CINDEX:
                self._exercise_options()
        except Exception as e:
            print(f"  ⚠ Campaign swap_option failed at cindex {cindex}: {e}")

    def _maybe_update_proposals(self):
        """Update all tracked proposal states so they can advance through the lifecycle."""
        if not self._proposal_ids:
            return
        updater = self.pool.agents[0].account
        for pid in self._proposal_ids:
            try:
                self.client.update_proposal_state(updater, pid)
            except Exception:
                pass
        self.pool._wait()

    def _submit_proposals(self):
        """Fund community treasury and submit swap-native-option proposals."""
        merchants = [a for a in self.pool.agents if a.has_business][:2]
        assert len(merchants) >= 2, "need at least 2 merchants for swap option campaign"
        self._merchants = merchants

        # Fund the community treasury with native tokens
        treasury = self.client.get_treasury(cid=self.cid)
        print(f"🏦 Campaign swap_option: treasury account = {treasury}")

        # Transfer native tokens from Alice to treasury
        print(f"  funding treasury with {self.TREASURY_FUND} native tokens")
        self.client.transfer(None, "//Alice", treasury, str(self.TREASURY_FUND))
        self.pool._wait()

        native_bal = self.client.balance(treasury)
        assert native_bal >= self.TREASURY_FUND, f"treasury funding failed: balance {native_bal}"
        print(f"  ✓ treasury native balance: {native_bal}")

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
        assert len(self._proposal_ids) == 2, (
            f"expected 2 swap-native-option proposals, got {len(self._proposal_ids)}")
        print(f"  ✓ submitted {len(self._proposal_ids)} proposals: {self._proposal_ids}")

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
            # Build vote tasks, then submit in parallel
            vote_tasks = []
            for voter in voters:
                reputations = [[t[1], t[0]] for t in self.client.reputation(voter.account)]
                if reputations:
                    vote_tasks.append((voter.account, proposal.id, reputations))

            def cast_vote(task):
                self.client.vote(task[0], task[1], 'aye', task[2])

            voted = 0
            with ThreadPoolExecutor(max_workers=100) as pool:
                futures = {pool.submit(cast_vote, task): task for task in vote_tasks}
                for future in as_completed(futures):
                    try:
                        future.result()
                        voted += 1
                    except Exception:
                        pass
            print(f"  proposal {proposal.id}: {voted} aye votes cast")
            self.pool._wait()

    def _update_and_check(self):
        """Update proposal states and check for approval."""
        assert self._proposal_ids, "swap option campaign: no proposals to update"

        print("📋 Campaign swap_option: updating proposal states")
        updater = self.pool.agents[0].account
        for pid in self._proposal_ids:
            self.client.update_proposal_state(updater, pid)
        self.pool._wait()

        proposals = self.client.get_proposals()
        for p in proposals:
            if p.id in self._proposal_ids:
                print(f"  proposal {p.id}: state={p.state}, turnout={p.turnout}, approval={p.approval}")
                assert p.state in ('Approved', 'Confirming', 'Ongoing'), (
                    f"unexpected state {p.state} for proposal {p.id}")

    def _exercise_options(self):
        """After enactment: query swap options and exercise them partially."""
        assert self._merchants, "swap option campaign: no merchants (submit phase failed?)"

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
        assert len(enacted) == len(self._proposal_ids), (
            f"expected {len(self._proposal_ids)} enacted proposals, got {len(enacted)}")
        print(f"  ✓ {len(enacted)} / {len(self._proposal_ids)} proposals enacted")

        exercised = 0
        for merchant in self._merchants:
            # Query the swap option
            option_str = self.client.get_swap_native_option(merchant.account, cid=self.cid)
            print(f"  {merchant.account[:8]}... option: {option_str[:120]}")
            assert "No swap" not in option_str, (
                f"no swap option found for {merchant.account[:8]}... after enactment")

            # Exercise half the allowance
            exercise_amount = self.NATIVE_ALLOWANCE // 2
            print(f"    exercising {exercise_amount} of {self.NATIVE_ALLOWANCE}")
            result = self.client.swap_native(merchant.account, exercise_amount, cid=self.cid)
            print(f"    ✓ swap result: {result}")
            exercised += 1

            self.pool._wait()

            # Query remaining option — should still exist with reduced allowance
            remaining = self.client.get_swap_native_option(merchant.account, cid=self.cid)
            assert "No swap" not in remaining, "option disappeared after partial exercise"
            print(f"    ✓ remaining option: {remaining[:120]}")

        assert exercised == len(self._merchants), (
            f"expected {len(self._merchants)} exercises, got {exercised}")

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
