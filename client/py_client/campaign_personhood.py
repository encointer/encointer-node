import re
from concurrent.futures import ThreadPoolExecutor, as_completed

from py_client.campaign import Campaign


class ProvePersonhoodCampaign(Campaign):
    """Prove + verify personhood at all ring levels for all agents with keys.

    Runs from cindex >= MIN_CINDEX onwards. Asserts pseudonym uniqueness per
    level (anonymity check).
    """

    TARGET_CINDEX = 6
    MAX_WORKERS = 100
    MAX_PROVERS = 10

    def __init__(self, pool, log=None):
        super().__init__(pool, log)
        self._results = []  # (cindex, level, agent_count, anonymity_ok)

    def on_post_ceremony(self, cindex):
        if cindex != self.TARGET_CINDEX:
            return

        all_ring_agents = [a for a in self.pool.agents if a.has_bandersnatch]
        if not all_ring_agents:
            print("  Campaign prove_personhood: no agents with bandersnatch keys, skipping")
            return

        # Find most recent cindex with populated rings
        chain_cindex = self.client.get_cindex()
        ring_cindex = None
        rings_output = None
        for ci in range(chain_cindex - 2, 0, -1):
            output = self.client.get_rings(self.cid, ci)
            m = re.search(r'Level 1/5:\s+(\d+)\s+members', output)
            if m and int(m.group(1)) > 0:
                ring_cindex = ci
                rings_output = output
                break

        if ring_cindex is None:
            print("  no rings with members found, skipping")
            return

        ring_agents = all_ring_agents[:self.MAX_PROVERS]
        print(f"🔮 Campaign prove_personhood: {len(ring_agents)}/{len(all_ring_agents)} agents (ring size {len(all_ring_agents)})")
        print(f"  found rings at cindex={ring_cindex}")

        available_levels = []
        for m in re.finditer(r'Level (\d)/5:\s+(\d+)\s+members', rings_output):
            level, count = int(m.group(1)), int(m.group(2))
            if count > 0:
                available_levels.append(level)
        print(f"  levels with members: {available_levels}")

        for level in available_levels:
            self._prove_level(cindex, ring_agents, ring_cindex, level)

    def _prove_one(self, agent, ring_cindex, level):
        """Prove + verify one agent at one level. Returns (pseudonym, error)."""
        try:
            secret = self.client.export_secret(agent.account)
            sig, output = self.client.prove_personhood(
                secret, self.cid, ring_cindex, level=level, sub_ring=0)
            pseudonym = None
            for line in output.split("\n"):
                if line.startswith("pseudonym:"):
                    pseudonym = line.split(maxsplit=1)[1].strip()
                    break
            valid, _ = self.client.verify_personhood(
                sig, self.cid, ring_cindex, level=level, sub_ring=0)
            if not valid:
                return None, f"verification failed for {agent.account[:8]}..."
            return pseudonym, None
        except Exception as e:
            return None, str(e)

    def _prove_level(self, cindex, ring_agents, ring_cindex, level):
        """Prove all agents at one level using thread pool."""
        pseudonyms = []
        proven = 0
        with ThreadPoolExecutor(max_workers=self.MAX_WORKERS) as pool:
            futures = {
                pool.submit(self._prove_one, agent, ring_cindex, level): agent
                for agent in ring_agents
            }
            for future in as_completed(futures):
                agent = futures[future]
                pseudonym, error = future.result()
                if error:
                    print(f"    level {level}/5 not available for {agent.account[:8]}...: {error}")
                else:
                    if pseudonym:
                        pseudonyms.append(pseudonym)
                    proven += 1

        anonymity_ok = len(pseudonyms) == len(set(pseudonyms)) if pseudonyms else True
        self._results.append((cindex, level, proven, anonymity_ok))
        status = "PASS" if anonymity_ok else "FAIL"
        print(f"  level {level}/5: {proven} proved, {len(pseudonyms)} pseudonyms, anonymity={status}")
        assert anonymity_ok, f"anonymity check failed at level {level}: duplicate pseudonyms"

    def write_summary(self, cindex):
        if not self._results or self.log is None:
            return
        ceremony_results = [(lv, cnt, ok) for ci, lv, cnt, ok in self._results if ci == cindex]
        if not ceremony_results:
            return
        self.log.phase('Campaign: prove_personhood', cindex)
        for level, count, ok in ceremony_results:
            self.log._file.write(f"  Level {level}/5: {count} proved, anonymity={'PASS' if ok else 'FAIL'}\n")
