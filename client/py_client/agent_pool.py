import ast
import random
import re
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from math import floor

from py_client.agents import Agent, AgentRole
from py_client.client import Client, ExtrinsicFeePaymentImpossible, ParticipantAlreadyLinked


MAX_WORKERS = 100


NUMBER_OF_ENDORSEMENTS_PER_REGISTRATION = 10


class AgentPool:
    def __init__(self, client: Client, cid: str, faucet_url: str, max_population: int,
                 waiting_blocks: int = 1, seed: int = 42):
        self.client = client
        self.cid = cid
        self.faucet_url = faucet_url
        self.max_population = max_population
        self.waiting_blocks = waiting_blocks
        self.rng = random.Random(seed)
        self.agents: list[Agent] = []
        self.stats: list[dict] = []
        self._faucet_account = None  # set during faucet lifecycle test
        self._heartbeat_account = None
        self._heartbeat_stop = None
        self._heartbeat_thread = None

    def _wait(self, blocks=None):
        self.client.await_block(blocks or self.waiting_blocks)

    # ── Bootstrap ──────────────────────────────────────────────────────

    def load_agents(self):
        """Load agents from existing keystore accounts (keys already registered on-chain)."""
        accounts = self.client.list_accounts()
        for acc in accounts:
            if not any(a.account == acc for a in self.agents):
                agent = Agent(account=acc, role=AgentRole.BOOTSTRAPPER)
                agent.bandersnatch_key = "auto-derived"
                agent.has_offline_identity = True
                self.agents.append(agent)

    def bootstrap(self, count: int):
        """Create initial bootstrapper accounts, fund them, and register bandersnatch keys."""
        accounts = self.client.create_accounts(count)
        print(f'created bootstrappers: {" ".join(accounts)}')
        self.client.faucet(accounts, faucet_url=self.faucet_url)
        agents = []
        for acc in accounts:
            agent = Agent(account=acc, role=AgentRole.BOOTSTRAPPER)
            agents.append(agent)
            self.agents.append(agent)
        self._register_keys_and_identities(agents)
        return accounts

    # ── Growth ─────────────────────────────────────────────────────────

    def _bootstrappers(self):
        return [a for a in self.agents if a.can_endorse]

    def _all_accounts(self):
        return [a.account for a in self.agents]

    def grow(self):
        """Endorse newcomers and create newbie accounts."""
        bootstrappers_raw = self.client.get_bootstrappers_with_remaining_newbie_tickets(self.cid)
        bootstrappers_with_tickets = ast.literal_eval(bootstrappers_raw)
        print(f'Bootstrappers with remaining newbie tickets {bootstrappers_with_tickets}')

        endorsees = self._endorse_new_accounts(bootstrappers_with_tickets, NUMBER_OF_ENDORSEMENTS_PER_REGISTRATION)

        if len(endorsees) > 0:
            print(f'Awaiting endorsement process')
            self._wait()
            print(f'Added endorsees to community: {len(endorsees)}')

        current_pop = len(self.agents)
        newbie_count = min(
            floor(current_pop / 1.5),
            self.max_population - current_pop
        )
        newbies = self.client.create_accounts(newbie_count)
        print(f'Add newbies to community {len(newbies)}')

        new_members = newbies + endorsees
        if new_members:
            self.client.faucet(new_members, faucet_url=self.faucet_url)
            self._wait()
            print(f'Fauceted new community members {len(new_members)}')

        new_agents = []
        for acc in endorsees:
            agent = Agent(account=acc, role=AgentRole.NEWBIE, endorsed=True)
            new_agents.append(agent)
            self.agents.append(agent)
        for acc in newbies:
            agent = Agent(account=acc, role=AgentRole.NEWBIE)
            new_agents.append(agent)
            self.agents.append(agent)

        if new_agents:
            self._register_keys_and_identities(new_agents)

    def _endorse_new_accounts(self, bootstrappers_and_tickets, endorsee_count):
        endorsers = []
        e_count = endorsee_count
        effective_endorsements = 0

        for bootstrapper, remaining_tickets in bootstrappers_and_tickets:
            tickets = min(remaining_tickets, e_count)
            if tickets > 0:
                endorsers.append((bootstrapper, tickets))
                effective_endorsements += tickets
            e_count -= tickets
            if e_count <= 0:
                break

        if effective_endorsements == 0:
            print("Can't endorse anymore, all tickets have been spent.")
            return []

        endorsees = self.client.create_accounts(effective_endorsements)
        start = 0
        for endorser, endorsement_count in endorsers:
            end = start + endorsement_count
            print(f'bootstrapper {endorser} endorses {endorsement_count} accounts.')
            self.client.endorse_newcomers(self.cid, endorser, endorsees[start:end])
            start += endorsement_count

        return endorsees

    # ── Heartbeat ─────────────────────────────────────────────────────

    def init_heartbeat(self):
        """Pre-create and fund the heartbeat account so start_heartbeat() is instant."""
        if self._heartbeat_account is None:
            self._heartbeat_account = self.client.create_accounts(1)[0]
            self.client.faucet([self._heartbeat_account], faucet_url=self.faucet_url)
            self._wait()

    def start_heartbeat(self):
        """Send periodic native transfers to prevent phase.py idle-block detection.

        Uses a dedicated account to avoid nonce clashes with agent or faucet accounts.
        """
        if self._heartbeat_thread is not None:
            return  # already running
        self.init_heartbeat()
        src = self._heartbeat_account
        dst = self.agents[0].account if self.agents else src
        stop_evt = threading.Event()

        def beat():
            while True:
                try:
                    self.client.transfer(None, src, dst, "1")
                except Exception:
                    pass
                if stop_evt.wait(4):
                    break

        t = threading.Thread(target=beat, daemon=True)
        t.start()
        self._heartbeat_stop = stop_evt
        self._heartbeat_thread = t

    def stop_heartbeat(self):
        """Stop the heartbeat thread."""
        if self._heartbeat_stop is not None:
            self._heartbeat_stop.set()
        if self._heartbeat_thread is not None:
            self._heartbeat_thread.join(timeout=5)
        self._heartbeat_stop = None
        self._heartbeat_thread = None

    # ── Parallel helpers ───────────────────────────────────────────────

    def _run_parallel(self, fn, items, max_workers=MAX_WORKERS):
        """Execute fn(item) concurrently. Returns count of successes."""
        succeeded = 0
        with ThreadPoolExecutor(max_workers=max_workers) as pool:
            futures = {pool.submit(fn, item): item for item in items}
            for future in as_completed(futures):
                try:
                    future.result()
                    succeeded += 1
                except Exception:
                    pass
        return succeeded

    # ── Phase execution ────────────────────────────────────────────────

    def execute_registering(self):
        """Claim rewards, grow population, register keys/identities, register all."""
        print("🏆 all participants claim their potential reward")
        self._run_parallel(lambda a: self.client.claim_reward(a.account, self.cid), self.agents)
        self._wait()

        self._update_proposal_states()

        total_supply = self._write_current_stats()
        if total_supply > 0:
            self.grow()

        self._register_all()
        self._wait()

    def execute_assigning(self):
        """Log meetup assignments, submit democracy proposals."""
        meetups = self.client.list_meetups(self.cid)
        meetup_sizes = list(map(len, meetups))
        print(f'🔎 meetups assigned for {sum(meetup_sizes)} participants with sizes: {meetup_sizes}')
        self._update_proposal_states()
        self.write_assigning_summary(len(meetups), meetup_sizes)
        self.write_democracy_summary()
        self._submit_democracy_proposals()

    def execute_attesting(self):
        """Perform meetups and vote."""
        meetups = self.client.list_meetups(self.cid)
        self._update_proposal_states()
        self._vote_on_proposals()
        print(f'🫂 Performing {len(meetups)} meetups')

        # Flatten all attestation tasks and run in parallel
        attest_tasks = []
        for meetup in meetups:
            for i in range(len(meetup)):
                attest_tasks.append((meetup[i], meetup[:i] + meetup[i + 1:]))
        self._run_parallel(
            lambda task: self.client.attest_attendees(task[0], self.cid, task[1]),
            attest_tasks
        )
        self._wait()

        # Track ceremony attendance for agents in meetups
        meetup_accounts = set()
        for meetup in meetups:
            meetup_accounts.update(meetup)
        for agent in self.agents:
            if agent.account in meetup_accounts:
                agent.ceremony_count += 1
                agent.promote()

    # ── Setup & base auxiliary features ─────────────────────────────

    def _setup_vk(self):
        """Set the offline payment verification key and faucet reserve amount (once)."""
        print("🔐 Setting offline payment verification key")
        self.client.set_offline_payment_vk(signer="//Alice")
        print("  verification key set")
        self.client.set_faucet_reserve_amount("//Alice", 0)
        print("  faucet reserve amount set to 0")

    def _setup_bazaar(self):
        """Create 5 merchant businesses and offerings."""
        print("🏪 Bazaar: creating businesses and offerings")
        ipfs_cid = "QmDUMMYikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"
        merchants = [a for a in self.agents if a.is_reputable][:5]
        for agent in merchants:
            self.client.create_business(agent.account, self.cid, ipfs_cid)
            agent.has_business = True
            agent.role = AgentRole.MERCHANT
            print(f"  created business for {agent.account[:8]}...")
        self._wait()
        for agent in merchants:
            self.client.create_offering(agent.account, self.cid, ipfs_cid)
            print(f"  created offering for {agent.account[:8]}...")
        self._wait()
        businesses = self.client.list_businesses(self.cid)
        print(f"  businesses: {businesses}")
        offerings = self.client.list_offerings(self.cid)
        print(f"  offerings: {offerings}")

    def _register_keys_and_identities(self, agents):
        """Register bandersnatch keys and offline identities for agents."""
        need_keys = [a for a in agents if not a.has_bandersnatch]
        need_ids = [a for a in agents if not a.has_offline_identity]

        def reg_key(agent):
            secret = self.client.export_secret(agent.account)
            self.client.register_bandersnatch_key(secret)
            agent.bandersnatch_key = "auto-derived"

        def reg_id(agent):
            self.client.register_offline_identity(agent.account, cid=self.cid)
            agent.has_offline_identity = True

        keys = self._run_parallel(reg_key, need_keys)
        ids = self._run_parallel(reg_id, need_ids)
        if keys or ids:
            self._wait()
        print(f"🔑 Registered {keys} bandersnatch keys, {ids} offline identities")

    def run_base_auxiliary(self, cindex):
        """Run base auxiliary feature exercises staged by ceremony index."""
        if cindex == 1:
            self._setup_vk()
        elif cindex == 2:
            self._setup_bazaar()
        elif cindex == 4:
            self._aux_transfers()
        elif cindex == 5:
            self._aux_faucet_lifecycle()
            self._aux_treasury()
            self._aux_queries()
        elif cindex == 6:
            self._aux_advanced_democracy()
        elif cindex >= 7:
            self._aux_queries()

    def _aux_transfers(self):
        """Ceremony 4: CC transfers between agents."""
        print("💰 Transfers between agents")
        reputables = [a for a in self.agents if a.is_reputable]
        assert len(reputables) >= 2, "need at least 2 reputables for transfer test"
        src, dst = reputables[0], reputables[1]
        bal_before = self.client.balance(dst.account, cid=self.cid)
        self.client.transfer(self.cid, src.account, dst.account, "0.1")
        self._wait()
        bal_after = self.client.balance(dst.account, cid=self.cid)
        assert bal_after > bal_before, f"transfer failed: dst balance {bal_before} -> {bal_after}"
        print(f"  ✓ transferred 0.1 from {src.account[:8]}... to {dst.account[:8]}...")

    def _aux_faucet_lifecycle(self):
        """Ceremony 5: Create, drip, close faucet."""
        print("🚰 Faucet lifecycle")
        creator = self._first_reputable()
        if not creator:
            print("  ⚠ no reputable agent, skipping faucet lifecycle")
            return
        try:
            self.client.transfer(None, "//Eve", creator.account, "100000")
            self._wait()
            cindex = self.client.get_cindex()
            whitelist = [self.cid]
            output = self.client.create_faucet(creator.account, "test-faucet", 10000, 1000, whitelist)
            print(f"  created faucet: {output[:80]}...")
            self._wait()

            faucets = self.client.list_faucets(verbose=True)
            if not faucets:
                print("  ⚠ no faucets found after creation — skipping drip/close")
                return
            print(f"  ✓ faucets exist: {faucets[:200]}...")

            drip_target = self.agents[-1] if len(self.agents) > 1 else self.agents[0]
            self.client.drip_faucet(drip_target.account, creator.account, cindex, cid=self.cid)
            print(f"  ✓ dripped to {drip_target.account[:8]}...")

            self.client.close_faucet(creator.account, creator.account)
            print("  ✓ closed faucet")
        except Exception as e:
            print(f"  ⚠ faucet lifecycle failed: {e}")

    def _aux_treasury(self):
        """Ceremony 5: Query treasury."""
        print("🏛 Treasury")
        treasury = self.client.get_treasury(cid=self.cid)
        assert len(treasury) > 0, "treasury account should not be empty"
        print(f"  ✓ treasury: {treasury}")

    def _aux_queries(self):
        """Ceremony 5+: Various read queries."""
        print("🔍 Running read queries")
        issuance = self.client.issuance(self.cid)
        assert float(issuance.split()[-1]) > 0, f"issuance should be positive, got: {issuance}"
        print(f"  ✓ issuance: {issuance}")
        reputables = self.client.list_reputables()
        assert len(reputables) > 0, "expected reputables"
        print(f"  ✓ reputables: {reputables[:200]}...")
        commitments = self.client.list_commitments()
        print(f"  commitments: {commitments[:200]}...")
        purposes = self.client.list_purposes()
        print(f"  purposes: {purposes[:200]}...")

    def _aux_advanced_democracy(self):
        """Ceremony 6: Advanced democracy proposals and voting."""
        print("🗳 Advanced democracy")
        proposer = self._first_reputable()
        assert proposer, "need at least one reputable for democracy"
        proposals_before = len(self.client.get_proposals())
        self.client.submit_update_demurrage_proposal(proposer.account, 1000000, cid=self.cid)
        print("  submitted demurrage proposal")
        self.client.submit_spend_native_proposal(proposer.account, self.agents[-1].account, 100)
        print("  submitted spend native proposal")
        self._wait()
        proposals_after = self.client.get_proposals()
        assert len(proposals_after) >= proposals_before + 2, (
            f"expected 2 new proposals, had {proposals_before}, now {len(proposals_after)}")
        print(f"  ✓ {len(proposals_after) - proposals_before} new proposals submitted")
        self._vote_on_proposals()
        self._update_proposal_states()
        enactment = self.client.list_enactment_queue()
        print(f"  enactment queue: {enactment[:200]}...")

    # ── Helpers ─────────────────────────────────────────────────────────

    def _first_reputable(self):
        for a in self.agents:
            if a.is_reputable:
                return a
        return None

    def _register_all(self):
        accounts = self._all_accounts()
        print(f'registering {len(accounts)} participants')
        need_refunding = []
        lock = threading.Lock()

        def register(p):
            try:
                self.client.register_participant(p, self.cid)
            except ExtrinsicFeePaymentImpossible:
                with lock:
                    need_refunding.append(p)
            except ParticipantAlreadyLinked:
                pass

        self._run_parallel(register, accounts)

        if need_refunding:
            print(f'the following accounts are out of funds and will be refunded {need_refunding}')
            self.client.faucet(need_refunding, faucet_url=self.faucet_url)
            self._wait()
            self._run_parallel(
                lambda p: self.client.register_participant(p, self.cid),
                need_refunding
            )

    def _submit_democracy_proposals(self):
        print("submitting new democracy proposals")
        proposer = self._first_reputable()
        if proposer:
            self.client.submit_update_nominal_income_proposal(proposer.account, 1.1, cid=self.cid)

    def _vote_on_proposals(self):
        proposals = self.client.get_proposals()
        for proposal in proposals:
            print(
                f"checking proposal {proposal.id}, state: {proposal.state}, "
                f"approval: {proposal.approval} turnout: {proposal.turnout}")
            if proposal.state == 'Ongoing' and proposal.turnout <= 1:
                choices = ['aye', 'nay']
                target_approval = self.rng.random()
                target_turnout = self.rng.random()
                print(
                    f"🗳 voting on proposal {proposal.id} with target approval of "
                    f"{target_approval * 100:.0f}% and target turnout of {target_turnout * 100:.0f}%")
                weights = [target_approval, 1 - target_approval]
                try:
                    active_voters = self._all_accounts()[0:round(len(self.agents) * target_turnout)]
                    print(f"will attempt to vote with {len(active_voters) - 1} accounts")

                    # Pre-compute votes and reputations, then submit in parallel
                    vote_tasks = []
                    is_first = True
                    for voter in active_voters:
                        reputations = [[t[1], t[0]] for t in self.client.reputation(voter)]
                        if not reputations:
                            print(f"no reputations for {voter}. can't vote")
                            continue
                        if is_first:
                            print(f"👉 will not vote with {voter}: mnemonic: {self.client.export_secret(voter)}")
                            is_first = False
                        vote = self.rng.choices(choices, weights)[0]
                        print(f"voting {vote} on proposal {proposal.id} with {voter} and reputations {reputations}")
                        vote_tasks.append((voter, proposal.id, vote, reputations))

                    self._run_parallel(
                        lambda t: self.client.vote(t[0], t[1], t[2], t[3]),
                        vote_tasks
                    )
                except Exception:
                    print(f"voting failed")
            self._wait()

    def _update_proposal_states(self):
        proposals = self.client.get_proposals()
        for proposal in proposals:
            print(
                f"checking proposal {proposal.id}, state: {proposal.state}, "
                f"approval: {proposal.approval} turnout: {proposal.turnout}")
            if proposal.state in ['Ongoing', 'Confirming']:
                print(f"updating proposal {proposal.id}")
                self.client.update_proposal_state(self.agents[0].account, proposal.id)

    # ── Stats & assertions ─────────────────────────────────────────────

    def _write_current_stats(self):
        accounts = self._all_accounts()
        with ThreadPoolExecutor(max_workers=MAX_WORKERS) as pool:
            bal = list(pool.map(lambda a: self.client.balance(a, cid=self.cid), accounts))
        total = sum(bal)
        print(f'****** money supply is {total}')

        businesses = sum(1 for a in self.agents if a.has_business)
        offline_ids = sum(1 for a in self.agents if a.has_offline_identity)
        ring_members = sum(1 for a in self.agents if a.bandersnatch_key is not None)

        stat = {
            'population': len(accounts),
            'total_supply': round(total),
            'businesses': businesses,
            'offline_ids': offline_ids,
            'ring_members': ring_members,
        }
        self.stats.append(stat)
        return total

    def write_assigning_summary(self, num_meetups, meetup_sizes):
        """Write newcomer and meetup stats to the simulation log."""
        if self.client.log is None:
            return
        newbies = sum(1 for a in self.agents if a.role == AgentRole.NEWBIE and not a.endorsed)
        endorsees = sum(1 for a in self.agents if a.role == AgentRole.NEWBIE and a.endorsed)
        lines = (
            f"  Newbies:    {newbies}\n"
            f"  Endorsees:  {endorsees}\n"
            f"  Meetups:    {num_meetups} ({sum(meetup_sizes)} participants, sizes {meetup_sizes})"
        )
        self.client.log.phase('Assigning Summary')
        self.client.log._file.write(lines + '\n')

    def write_democracy_summary(self):
        """Write a democracy summary to the simulation log."""
        if self.client.log is None:
            return
        proposals = self.client.get_proposals()
        if not proposals:
            self.client.log.phase('Democracy: no proposals')
            return
        by_state = {}
        for p in proposals:
            by_state.setdefault(p.state, []).append(p)
        lines = []
        for state in ['Ongoing', 'Confirming', 'Approved', 'Rejected', 'SupersededBy', 'Enacted']:
            group = by_state.get(state, [])
            if group:
                lines.append(f"  {state}: {len(group)}")
                for p in group:
                    lines.append(f"    #{p.id} {p.action}  turnout={p.turnout} approval={p.approval}")
        self.client.log.phase('Democracy')
        self.client.log._file.write('\n'.join(lines) + '\n')

    def write_ceremony_summary(self, cindex):
        """Write a human-readable ceremony summary to the simulation log."""
        if self.client.log is None:
            return

        population = len(self.agents)
        reputables = sum(1 for a in self.agents if a.is_reputable)
        businesses = sum(1 for a in self.agents if a.has_business)
        offline_ids = sum(1 for a in self.agents if a.has_offline_identity)
        ring_members = sum(1 for a in self.agents if a.bandersnatch_key is not None)

        # Try to find rings
        rings_info = "none yet"
        chain_cindex = self.client.get_cindex()
        for ci in range(chain_cindex - 2, 0, -1):
            rings_output = self.client.get_rings(self.cid, ci)
            levels = re.findall(r'Level (\d)/5:\s+(\d+)\s+members', rings_output)
            if levels and any(int(c) > 0 for _, c in levels):
                parts = [f"L{lv}/5={ct}" for lv, ct in levels]
                rings_info = f"cindex={ci}: {', '.join(parts)}"
                break

        aux_map = {
            1: "setup-vk",
            2: "setup-bazaar",
            4: "transfers",
            5: "faucet-lifecycle, treasury, queries",
            6: "advanced-democracy",
        }
        aux = aux_map.get(cindex, "queries" if cindex >= 7 else "none")

        total_supply = self.stats[-1]['total_supply'] if self.stats else 0

        text = (
            f"  Population:         {population}\n"
            f"  Reputables:         {reputables}\n"
            f"  Money supply:       {total_supply}\n"
            f"  Businesses:         {businesses}\n"
            f"  Offline identities: {offline_ids}\n"
            f"  Ring members:       {ring_members}\n"
            f"  Rings: {rings_info}\n"
            f"  Auxiliary features: {aux}"
        )
        self.client.log.summary(text)

    def write_stats(self, path='bot-stats.csv'):
        with open(path, 'w') as f:
            for s in self.stats:
                f.write(f"{s['population']}, {s['total_supply']}, "
                        f"{s['businesses']}, {s['offline_ids']}, {s['ring_members']}\n")

    def assert_invariants(self, cindex, fail_fast=False, failures=None):
        """Per-ceremony assertions for CI mode."""
        stat = self.stats[-1] if self.stats else None
        if not stat:
            return

        if failures is None:
            failures = []

        def check(condition, msg):
            if not condition:
                tagged = f"[ceremony {cindex}] {msg}"
                if fail_fast:
                    assert False, tagged
                failures.append(tagged)
                print(f"  ✗ {tagged}")
                return False
            return True

        print(f"🔬 Asserting invariants for ceremony {cindex}")

        # Population should be positive and growing
        check(stat['population'] > 0, f"population is 0")
        if cindex >= 3:
            check(stat['population'] > 10, f"population should grow beyond bootstrappers")

        # All agents should have bandersnatch keys and offline identities (registered at creation)
        ok = check(stat['ring_members'] == stat['population'],
                   f"ring_members ({stat['ring_members']}) != population ({stat['population']})")
        ok = check(stat['offline_ids'] == stat['population'],
                   f"offline_ids ({stat['offline_ids']}) != population ({stat['population']})") and ok
        if ok:
            print(f"  ✓ {stat['ring_members']} keys, {stat['offline_ids']} offline ids == population")

        # After ceremony 1: total supply should be positive (bootstrappers earned income)
        if cindex >= 2:
            if check(stat['total_supply'] > 0, "total supply is 0"):
                print(f"  ✓ total supply {stat['total_supply']} > 0")

        # After ceremony 2: businesses should exist (5 merchants set up in ceremony 2)
        if cindex >= 3:
            businesses = self.client.list_businesses(self.cid)
            check(stat['businesses'] >= 5, f"expected >= 5 businesses, got {stat['businesses']}")
            if check(len(businesses) >= 5, f"expected >= 5 on-chain businesses, got {len(businesses)}"):
                print(f"  ✓ {len(businesses)} businesses on-chain")

        # Verify offline identity readable on-chain
        if cindex >= 3:
            offline_agents = [a for a in self.agents if a.has_offline_identity]
            identity = self.client.get_offline_identity(offline_agents[0].account, cid=self.cid)
            if check(len(identity) > 0, f"offline identity empty for {offline_agents[0].account[:8]}..."):
                print(f"  ✓ offline identity verified on-chain")

        # After ceremony 2: rings should exist with members matching population
        if cindex >= 3:
            chain_cindex = self.client.get_cindex()
            found_rings = False
            for ci in range(chain_cindex - 2, 0, -1):
                rings = self.client.get_rings(self.cid, ci)
                m = re.search(r'Level 1/5:\s+(\d+)\s+members', rings)
                if m and int(m.group(1)) > 0:
                    members = int(m.group(1))
                    print(f"  ✓ rings at cindex={ci}: level 1/5 has {members} members")
                    found_rings = True
                    break
            check(found_rings, f"no rings with members found")

        # After ceremony 5: democracy proposals should have been voted on
        if cindex >= 5:
            proposals = self.client.get_proposals()
            check(len(proposals) > 0, "expected democracy proposals by ceremony 5")
            voted = [p for p in proposals if p.turnout > 0]
            if check(len(voted) > 0, "expected at least one proposal with votes"):
                print(f"  ✓ {len(proposals)} proposals, {len(voted)} with votes")

        if not any(f.startswith(f"[ceremony {cindex}]") for f in failures):
            print(f"  ✓ all invariants passed for ceremony {cindex}")
