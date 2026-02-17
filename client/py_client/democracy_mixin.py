from py_client.democracy import parse_proposals
from py_client.base import ensure_clean_exit


class _DemocracyMixin:
    def submit_set_inactivity_timeout_proposal(self, account, inactivity_timeout, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["submit-set-inactivity-timeout-proposal", account, str(inactivity_timeout)], cid,
                                   pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def submit_update_nominal_income_proposal(self, account, new_income, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["submit-update-nominal-income-proposal", account, str(new_income)], cid,
                                   pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def submit_update_demurrage_proposal(self, account, demurrage_halving_blocks, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["submit-update-demurrage-proposal", account, str(demurrage_halving_blocks)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def submit_petition(self, account, demand, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["submit-petition", account, demand], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def submit_spend_native_proposal(self, account, to, amount, pay_fees_in_cc=False):
        ret = self.run_cli_command(["submit-spend-native-proposal", account, to, str(amount)],
                                   pay_fees_in_cc=pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def vote(self, account, proposal_id, vote, reputations, cid=None, pay_fees_in_cc=False):
        reputations = [f'{cid}_{cindex}' for [cid, cindex] in reputations]
        reputation_vec = ','.join(reputations)
        ret = self.run_cli_command(["vote", account, str(proposal_id), vote, reputation_vec], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def update_proposal_state(self, account, proposal_id, cid=None, pay_fees_in_cc=False):
        ret = self.run_cli_command(["update-proposal-state", account, str(proposal_id)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def list_proposals(self):
        ret = self.run_cli_command(["list-proposals"])
        return ret.stdout.decode("utf-8").strip()

    def get_proposals(self):
        return parse_proposals(self.list_proposals())

    def list_enactment_queue(self):
        ret = self.run_cli_command(["list-enactment-queue"])
        return ret.stdout.decode("utf-8").strip()
