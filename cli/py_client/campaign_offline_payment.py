import os
import tempfile
from concurrent.futures import ThreadPoolExecutor, as_completed

from py_client.campaign import Campaign


class OfflinePaymentCampaign(Campaign):
    """Mass offline payments + circular merchant economy + retry settlement.

    Runs at target_ceremony only.
    """

    MAX_RETRY_ROUNDS = 10

    def __init__(self, pool, log=None, target_ceremony=7):
        super().__init__(pool, log)
        self.target_ceremony = target_ceremony
        self._stats = None

    def on_post_ceremony(self, cindex):
        if cindex != self.target_ceremony:
            return
        try:
            self._run_offline_payments()
        except Exception as e:
            print(f"  ⚠ Campaign offline_payment failed: {e}")

    def _run_offline_payments(self):
        offline_agents = [a for a in self.pool.agents if a.has_offline_identity]
        merchants = [a for a in self.pool.agents if a.has_business]

        if len(offline_agents) < 2:
            print("  Campaign offline_payment: not enough offline agents, skipping")
            return

        print(f"💸 Campaign offline_payment: {len(offline_agents)} agents, {len(merchants)} merchants")

        pop_proofs = self._generate_population_payments(offline_agents)
        merchant_proofs = self._generate_merchant_cycle(merchants) if len(merchants) >= 2 else []

        all_proofs = pop_proofs + merchant_proofs
        self.pool.rng.shuffle(all_proofs)
        print(f"  total proofs: {len(all_proofs)} (pop={len(pop_proofs)}, merchant={len(merchant_proofs)})")

        self._stats = self._settle_with_retries(all_proofs)

        # Log sample balances
        sample = offline_agents[:5]
        for agent in sample:
            bal = self.client.balance(agent.account, cid=self.cid)
            print(f"  balance {agent.account[:8]}...: {bal:.2f}")

    def _generate_population_payments(self, offline_agents):
        """Each offline agent sends 3 random payments."""
        proofs = []
        for agent in offline_agents:
            recipients = self.pool.rng.choices(
                [a for a in offline_agents if a.account != agent.account],
                k=min(3, len(offline_agents) - 1))
            for recipient in recipients:
                amount = f"{self.pool.rng.uniform(0.01, 0.5):.2f}"
                try:
                    proof_json = self.client.generate_offline_payment(
                        signer=agent.account, to=recipient.account, amount=amount, cid=self.cid)
                    path = tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False)
                    path.write(proof_json)
                    path.close()
                    proofs.append(path.name)
                except Exception as e:
                    print(f"    proof gen failed {agent.account[:8]}...→{recipient.account[:8]}...: {e}")
        print(f"  population proofs generated: {len(proofs)}")
        return proofs

    def _generate_merchant_cycle(self, merchants):
        """Directed cycle: M0→M1→M2→...→M0 with turnover > individual balance."""
        n = len(merchants)
        balances = [self.client.balance(m.account, cid=self.cid) for m in merchants]
        avg = sum(balances) / n if n else 0
        amount = f"{0.8 * avg:.2f}"
        print(f"  merchant cycle: {n} merchants, avg balance={avg:.2f}, cycle amount={amount}")

        proofs = []
        for i in range(n):
            sender = merchants[i]
            recipient = merchants[(i + 1) % n]
            try:
                proof_json = self.client.generate_offline_payment(
                    signer=sender.account, to=recipient.account, amount=amount, cid=self.cid)
                path = tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False)
                path.write(proof_json)
                path.close()
                proofs.append(path.name)
            except Exception as e:
                print(f"    merchant proof failed {sender.account[:8]}...→{recipient.account[:8]}...: {e}")
        print(f"  merchant cycle proofs generated: {len(proofs)}")
        return proofs

    def _settle_with_retries(self, proof_paths):
        """Settle all proofs, parallel across settlers, sequential per settler."""
        settlers = [a.account for a in self.pool.agents if a.is_reputable]
        if not settlers:
            settlers = [self.pool.agents[0].account]
        print(f"  using {len(settlers)} settlers for parallel submission")

        pending = list(proof_paths)
        total = len(pending)

        # Bucket proofs by settler: each settler submits its batch sequentially
        buckets = {s: [] for s in settlers}
        for i, path in enumerate(pending):
            buckets[settlers[i % len(settlers)]].append(path)

        settled_count = 0
        failed_paths = []

        def settle_batch(settler, paths):
            ok, fail = 0, []
            for path in paths:
                try:
                    self.client.submit_offline_payment(signer=settler, proof_file=path)
                    os.unlink(path)
                    ok += 1
                except Exception:
                    fail.append(path)
            return ok, fail

        with ThreadPoolExecutor(max_workers=len(settlers)) as pool:
            futures = {pool.submit(settle_batch, s, paths): s
                       for s, paths in buckets.items() if paths}
            for future in as_completed(futures):
                ok, fail = future.result()
                settled_count += ok
                failed_paths.extend(fail)

        self.pool._wait()
        print(f"  settlement: {settled_count} settled, {len(failed_paths)} failed")
        pending = failed_paths

        # Clean up any remaining temp files
        for path in pending:
            try:
                os.unlink(path)
            except OSError:
                pass

        total_settled = total - len(pending)
        stats = {'total': total, 'settled': total_settled, 'failed': len(pending)}

        if pending:
            print(f"  ⚠ {len(pending)} proofs failed to settle")
        else:
            print(f"  all {total_settled}/{total} proofs settled")
        return stats

    def write_summary(self, cindex):
        if self._stats is None or self.log is None or cindex != self.target_ceremony:
            return
        s = self._stats
        self.log.phase('Campaign: offline_payment', cindex)
        self.log._file.write(
            f"  Total proofs: {s['total']}\n"
            f"  Settled:      {s['settled']}\n"
            f"  Failed:       {s['failed']}\n")
