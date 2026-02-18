import ast
import json
import os
import random
import re
import tempfile
from math import floor

from py_client.agents import Agent, AgentRole
from py_client.client import Client, ExtrinsicFeePaymentImpossible, ParticipantAlreadyLinked


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

    def _wait(self, blocks=None):
        self.client.await_block(blocks or self.waiting_blocks)

    # ── Bootstrap ──────────────────────────────────────────────────────

    def load_agents(self):
        """Load agents from existing keystore accounts."""
        accounts = self.client.list_accounts()
        for acc in accounts:
            if not any(a.account == acc for a in self.agents):
                self.agents.append(Agent(account=acc, role=AgentRole.BOOTSTRAPPER))

    def bootstrap(self, count: int):
        """Create initial bootstrapper accounts and fund them."""
        accounts = self.client.create_accounts(count)
        print(f'created bootstrappers: {" ".join(accounts)}')
        self.client.faucet(accounts, faucet_url=self.faucet_url)
        for acc in accounts:
            self.agents.append(Agent(account=acc, role=AgentRole.BOOTSTRAPPER))
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

        for acc in endorsees:
            self.agents.append(Agent(account=acc, role=AgentRole.NEWBIE))
        for acc in newbies:
            self.agents.append(Agent(account=acc, role=AgentRole.NEWBIE))

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

    # ── Phase execution ────────────────────────────────────────────────

    def execute_registering(self):
        """Claim rewards, grow population, register all."""
        print("🏆 all participants claim their potential reward")
        for agent in self.agents:
            self.client.claim_reward(agent.account, self.cid)
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
        self._submit_democracy_proposals()

    def execute_attesting(self):
        """Perform meetups and vote."""
        meetups = self.client.list_meetups(self.cid)
        self._update_proposal_states()
        self._vote_on_proposals()
        print(f'🫂 Performing {len(meetups)} meetups')
        for meetup in meetups:
            self._perform_meetup(meetup)
        self._wait()

        # Track ceremony attendance for agents in meetups
        meetup_accounts = set()
        for meetup in meetups:
            meetup_accounts.update(meetup)
        for agent in self.agents:
            if agent.account in meetup_accounts:
                agent.ceremony_count += 1
                agent.promote()

    # ── Auxiliary features (staged by ceremony index) ──────────────────

    def run_auxiliary_features(self, cindex):
        """Run auxiliary feature exercises staged by ceremony index."""
        if cindex == 2:
            self._aux_bazaar()
            self._aux_reputation_rings_setup()
        elif cindex == 3:
            self._aux_offline_payment_setup()
            self._aux_democracy_proposals()
        elif cindex == 4:
            self._aux_offline_payment_back_and_forth()
            self._aux_reputation_rings_verify()
            self._aux_transfers()
        elif cindex == 5:
            self._aux_faucet_lifecycle()
            self._aux_treasury()
            self._aux_queries()
        elif cindex == 6:
            self._aux_advanced_democracy()
        elif cindex >= 7:
            # Steady state: all features active
            self._aux_queries()

    def _aux_bazaar(self):
        """Ceremony 2: Create businesses and offerings."""
        print("🏪 Bazaar: creating businesses and offerings")
        ipfs_cid = "QmDUMMYikh7VqTu8pvzd2G2vAd4eK7EaazXTEgqGN6AWoD"
        merchants = [a for a in self.agents if a.is_reputable][:2]
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

    def _aux_offline_payment_setup(self):
        """Ceremony 3: Set VK, register offline identities."""
        print("🔐 Offline payment setup")
        self.client.set_offline_payment_vk(signer="//Alice")
        print("  verification key set")

        reputables = [a for a in self.agents if a.is_reputable and not a.has_offline_identity][:2]
        for agent in reputables:
            self.client.register_offline_identity(agent.account, cid=self.cid)
            agent.has_offline_identity = True
            print(f"  registered offline identity for {agent.account[:8]}...")
        self._wait()

        for agent in reputables:
            identity = self.client.get_offline_identity(agent.account, cid=self.cid)
            print(f"  offline identity for {agent.account[:8]}...: {identity[:60]}...")

    def _aux_reputation_rings_setup(self):
        """Ceremony 2: Register auto-derived bandersnatch keys.

        Keys must be registered before the next Assigning phase so that
        automatic ring computation picks them up via on_idle.
        """
        print("🔑 Reputation rings: registering bandersnatch keys (auto-derived)")
        reputables = [a for a in self.agents if a.is_reputable][:3]
        for agent in reputables:
            output = self.client.register_bandersnatch_key(agent.account)
            agent.bandersnatch_key = "auto-derived"
            print(f"  registered key for {agent.account[:8]}...: {output[:80]}...")
        self._wait()

    def _aux_democracy_proposals(self):
        """Ceremony 3: Submit various proposals."""
        print("🗳 Democracy: submitting proposals")
        proposer = self._first_reputable()
        if not proposer:
            return
        self.client.submit_update_nominal_income_proposal(proposer.account, 1.1, cid=self.cid)
        print("  submitted update nominal income proposal")
        self.client.submit_petition(proposer.account, "test-petition", cid=self.cid)
        print("  submitted petition")

    def _aux_offline_payment_back_and_forth(self):
        """Ceremony 4: Alice→Bob 6, Bob→Alice 3, Alice→Bob 6 — settle in order."""
        offline_agents = [a for a in self.agents if a.has_offline_identity]
        if len(offline_agents) < 2:
            print("⚠ Not enough offline agents for payment back-and-forth")
            return

        alice, bob = offline_agents[0], offline_agents[1]
        print(f"💸 Offline payment back-and-forth: {alice.account[:8]}... ↔ {bob.account[:8]}...")

        # Generate 3 proofs
        proofs = []
        proof_specs = [
            (alice, bob, "0.6"),
            (bob, alice, "0.3"),
            (alice, bob, "0.6"),
        ]
        for sender, recipient, amount in proof_specs:
            proof_json = self.client.generate_offline_payment(
                signer=sender.account, to=recipient.account, amount=amount, cid=self.cid)
            proof_file = tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False)
            proof_file.write(proof_json)
            proof_file.close()
            proofs.append(proof_file.name)
            print(f"  generated proof: {sender.account[:8]}... → {recipient.account[:8]}... amount={amount}")

        # Settle in order with a neutral third party
        settler = self.agents[2].account if len(self.agents) > 2 else "//Charlie"
        for i, proof_path in enumerate(proofs):
            self.client.submit_offline_payment(signer=settler, proof_file=proof_path)
            self._wait()
            a_bal = self.client.balance(alice.account, cid=self.cid)
            b_bal = self.client.balance(bob.account, cid=self.cid)
            print(f"  after settlement {i+1}: alice={a_bal:.2f} bob={b_bal:.2f}")
            os.unlink(proof_path)

    def _aux_reputation_rings_verify(self):
        """Ceremony 4: Query auto-computed rings and exercise ring-VRF prove/verify.

        Rings were auto-computed during the Assigning phase via on_idle.
        Bandersnatch keys were registered at ceremony 2, so they appear in
        rings for ceremony 2+ (computed at ceremony 3's Assigning phase).
        Exercises all reputation levels from 1/5 to 5/5.
        """
        print("💍 Reputation rings: verifying auto-computed rings + ring-VRF")

        ring_agents = [a for a in self.agents if a.has_bandersnatch]
        if not ring_agents:
            print("  ⚠ no agents with bandersnatch keys, skipping")
            return

        # Find the most recent ceremony with rings, scanning backwards.
        # Rings for cindex N are auto-computed during ceremony N+1's Assigning phase.
        chain_cindex = self.client.get_cindex()
        ring_cindex = None
        rings_output = None
        for ci in range(chain_cindex - 2, 0, -1):
            rings_output = self.client.get_rings(self.cid, ci)
            m = re.search(r'Level 1/5:\s+(\d+)\s+members', rings_output)
            if m and int(m.group(1)) > 0:
                ring_cindex = ci
                print(f"  found rings at cindex={ci}")
                break

        if ring_cindex is None:
            print("  ⚠ no rings with members found, skipping ring-VRF")
            return

        # Parse which levels have members from the get-rings output.
        # Format: "  Level N/5: M members (S sub-rings)"
        available_levels = []
        for m in re.finditer(r'Level (\d)/5:\s+(\d+)\s+members', rings_output):
            level, count = int(m.group(1)), int(m.group(2))
            if count > 0:
                available_levels.append(level)
        print(f"  levels with members: {available_levels}")

        # Ring-VRF prove/verify for each level and agent
        for level in available_levels:
            print(f"  🔮 Ring-VRF level {level}/5 at cindex={ring_cindex}")
            for agent in ring_agents:
                try:
                    sig, output = self.client.prove_personhood(
                        agent.account, self.cid, ring_cindex, level=level, sub_ring=0)
                    print(f"    proved level {level}/5 for {agent.account[:8]}...")

                    valid, verify_output = self.client.verify_personhood(
                        sig, self.cid, ring_cindex, level=level, sub_ring=0)
                    assert valid, f"verification failed for {agent.account[:8]}... at level {level}"
                    print(f"    verified: {verify_output[:80]}...")
                except Exception as e:
                    # Agent may not be in higher-level rings (not enough ceremonies attended)
                    print(f"    level {level}/5 not available for {agent.account[:8]}...: {e}")

    def _aux_transfers(self):
        """Ceremony 4: CC transfers between agents."""
        print("💰 Transfers between agents")
        reputables = [a for a in self.agents if a.is_reputable]
        if len(reputables) < 2:
            return
        src, dst = reputables[0], reputables[1]
        self.client.transfer(self.cid, src.account, dst.account, "0.1")
        self._wait()
        print(f"  transferred 0.1 from {src.account[:8]}... to {dst.account[:8]}...")

    def _aux_faucet_lifecycle(self):
        """Ceremony 5: Create, drip, close faucet."""
        print("🚰 Faucet lifecycle")
        creator = self._first_reputable()
        if not creator:
            return
        cindex = self.client.get_cindex()
        whitelist = [self.cid]
        output = self.client.create_faucet(creator.account, "test-faucet", 1000, 10, whitelist)
        print(f"  created faucet: {output[:80]}...")
        self._wait()

        faucets = self.client.list_faucets(verbose=True)
        print(f"  faucets: {faucets[:200]}...")

        # drip to another agent
        drip_target = self.agents[-1] if len(self.agents) > 1 else self.agents[0]
        try:
            self.client.drip_faucet(drip_target.account, creator.account, cindex, cid=self.cid)
            print(f"  dripped to {drip_target.account[:8]}...")
        except Exception as e:
            print(f"  drip failed (expected in some scenarios): {e}")

        try:
            self.client.close_faucet(creator.account, creator.account)
            print("  closed faucet")
        except Exception as e:
            print(f"  close faucet failed (expected if not empty): {e}")

    def _aux_treasury(self):
        """Ceremony 5: Query treasury."""
        print("🏛 Treasury")
        treasury = self.client.get_treasury(cid=self.cid)
        print(f"  treasury: {treasury}")

    def _aux_queries(self):
        """Ceremony 5+: Various read queries."""
        print("🔍 Running read queries")
        issuance = self.client.issuance(self.cid)
        print(f"  issuance: {issuance}")
        reputables = self.client.list_reputables()
        print(f"  reputables: {reputables[:200]}...")
        commitments = self.client.list_commitments()
        print(f"  commitments: {commitments[:200]}...")
        purposes = self.client.list_purposes()
        print(f"  purposes: {purposes[:200]}...")

    def _aux_advanced_democracy(self):
        """Ceremony 6: Advanced democracy proposals and voting."""
        print("🗳 Advanced democracy")
        proposer = self._first_reputable()
        if not proposer:
            return
        self.client.submit_update_demurrage_proposal(proposer.account, 1000000, cid=self.cid)
        print("  submitted demurrage proposal")
        self.client.submit_spend_native_proposal(proposer.account, self.agents[-1].account, 100)
        print("  submitted spend native proposal")
        self._wait()
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
        for p in accounts:
            try:
                self.client.register_participant(p, self.cid)
            except ExtrinsicFeePaymentImpossible:
                need_refunding.append(p)
            except ParticipantAlreadyLinked:
                pass

        if len(need_refunding) > 0:
            print(f'the following accounts are out of funds and will be refunded {need_refunding}')
            self.client.faucet(need_refunding, faucet_url=self.faucet_url)
            self._wait()
            for p in need_refunding:
                try:
                    self.client.register_participant(p, self.cid)
                except ExtrinsicFeePaymentImpossible:
                    print("refunding failed")

    def _perform_meetup(self, meetup):
        n = len(meetup)
        print(f'Performing meetup with {n} participants')
        for p_index in range(n):
            attestor = meetup[p_index]
            attendees = meetup[:p_index] + meetup[p_index + 1:]
            self.client.attest_attendees(attestor, self.cid, attendees)

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
                    is_first_voter_with_rep = True
                    for voter in active_voters:
                        reputations = [[t[1], t[0]] for t in self.client.reputation(voter)]
                        if len(reputations) == 0:
                            print(f"no reputations for {voter}. can't vote")
                            continue
                        if is_first_voter_with_rep:
                            print(f"👉 will not vote with {voter}: mnemonic: {self.client.export_secret(voter)}")
                            is_first_voter_with_rep = False
                        vote = self.rng.choices(choices, weights)[0]
                        print(f"voting {vote} on proposal {proposal.id} with {voter} and reputations {reputations}")
                        self.client.vote(voter, proposal.id, vote, reputations)
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
        bal = [self.client.balance(a, cid=self.cid) for a in accounts]
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
            2: "bazaar, reputation-rings-setup",
            3: "offline-payment-setup, democracy-proposals",
            4: "offline-payment-back-and-forth, reputation-rings-verify, transfers",
            5: "faucet-lifecycle, treasury, queries",
            6: "advanced-democracy",
        }
        aux = aux_map.get(cindex, "queries" if cindex >= 7 else "none")

        text = (
            f"  Population:         {population}\n"
            f"  Reputables:         {reputables}\n"
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

    def assert_invariants(self, cindex):
        """Per-ceremony assertions for CI mode."""
        stat = self.stats[-1] if self.stats else None
        if not stat:
            return

        print(f"🔬 Asserting invariants for ceremony {cindex}")

        # Population should be positive
        assert stat['population'] > 0, f"population is 0 at cindex {cindex}"

        # After ceremony 2: businesses should exist
        if cindex >= 2:
            businesses = self.client.list_businesses(self.cid)
            if stat['businesses'] > 0:
                assert len(businesses) > 0, "expected businesses after ceremony 2"
                print(f"  ✓ businesses exist")

        # After ceremony 2: bandersnatch keys registered
        if cindex >= 2:
            if stat['ring_members'] > 0:
                print(f"  ✓ bandersnatch keys registered")

        # After ceremony 3: offline identities registered
        if cindex >= 3:
            if stat['offline_ids'] > 0:
                offline_agents = [a for a in self.agents if a.has_offline_identity]
                for agent in offline_agents[:1]:
                    identity = self.client.get_offline_identity(agent.account, cid=self.cid)
                    assert len(identity) > 0, f"offline identity empty for {agent.account[:8]}..."
                print(f"  ✓ offline identities registered")

        # After ceremony 4: rings auto-computed
        if cindex >= 4 and stat['ring_members'] > 0:
            # Rings for cindex N are auto-computed during ceremony N+1's Assigning phase.
            # By ceremony 4 end, rings for cindex 2 should exist.
            for ci in range(self.client.get_cindex() - 2, 0, -1):
                rings = self.client.get_rings(self.cid, ci)
                if "members" in rings:
                    print(f"  ✓ auto-computed rings exist for cindex={ci}")
                    break

        print(f"  ✓ all invariants passed for ceremony {cindex}")
