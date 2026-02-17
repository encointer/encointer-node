from py_client.base import ensure_clean_exit


class _CeremonyMixin:
    def register_participant(self, account, cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["register-participant", account], cid, pay_fees_in_cc)
        ensure_clean_exit(ret)

    def upgrade_registration(self, account, cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["upgrade-registration", account], cid, pay_fees_in_cc)
        ensure_clean_exit(ret)

    def unregister_participant(self, account, cid, cindex=None, pay_fees_in_cc=False):
        command = ["unregister-participant", account]
        if cindex:
            command += [str(cindex)]
        ret = self.run_cli_command(command, cid, pay_fees_in_cc)
        ensure_clean_exit(ret)

    def endorse_newcomers(self, cid, endorser, endorsees, pay_fees_in_cc=False):
        ret = self.run_cli_command(
            ["endorse-newcomers", endorser, "--endorsees"] +
            endorsees,  # must be separate to append a list of args to the cli
            cid,
            pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def get_bootstrappers_with_remaining_newbie_tickets(self, cid):
        ret = self.run_cli_command(["get-bootstrappers-with-remaining-newbie-tickets"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def attest_attendees(self, account, cid, attendees, pay_fees_in_cc=False):
        ret = self.run_cli_command(["attest-attendees", account] + attendees, cid=cid, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)

    def list_participants(self, cid):
        ret = self.run_cli_command(["list-participants"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_meetups(self, cid):
        ret = self.run_cli_command(["list-meetups"], cid)
        meetups = []
        lines = ret.stdout.decode("utf-8").splitlines()
        while len(lines) > 0:
            if 'participants:' in lines.pop(0):
                participants = []
                while len(lines) > 0:
                    l = lines.pop(0)
                    if ('MeetupRegistry' in l) or ('total' in l) or ('CSV:' in l):
                        break
                    participants.append(l.strip())
                meetups.append(participants)
        return meetups

    def list_attestees(self, cid):
        ret = self.run_cli_command(["list-attestees"], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def new_claim(self, account, vote, cid, pay_fees_in_cc=False):
        ret = self.run_cli_command(["new-claim", account, str(vote)], cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def claim_reward(self, account, cid, meetup_index=None, all=False, pay_fees_in_cc=False):
        optional_args = []
        if meetup_index:
            optional_args += ["--meetup-index", str(meetup_index)]
        if all:
            optional_args += ["--all"]

        ret = self.run_cli_command(["claim-reward", "--signer", account] + optional_args, cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def reputation(self, account):
        ret = self.run_cli_command(["reputation", account])
        ensure_clean_exit(ret)
        reputation_history = []
        lines = ret.stdout.decode("utf-8").splitlines()
        while len(lines) > 0:
            (cindex, cid, rep) = [item.strip() for item in lines.pop(0).split(',')]
            reputation_history.append(
                (cindex, cid, rep.strip().split('::')[1]))
        return reputation_history

    def purge_community_ceremony(self, cid, from_cindex, to_cindex, pay_fees_in_cc=False):
        ret = self.run_cli_command(["purge-community-ceremony", str(from_cindex), str(to_cindex)], cid, pay_fees_in_cc)
        return ret.stdout.decode("utf-8").strip()

    def print_ceremony_stats(self, cid, ceremony_index=None):
        cmd = ["print-ceremony-stats"]
        if ceremony_index is not None:
            cmd += ["--ceremony-index", str(ceremony_index)]
        ret = self.run_cli_command(cmd, cid=cid)
        return ret.stdout.decode("utf-8").strip()

    def list_reputables(self, verbose=False):
        cmd = ["list-reputables"]
        if verbose:
            cmd += ["-v"]
        ret = self.run_cli_command(cmd)
        return ret.stdout.decode("utf-8").strip()

    def get_proof_of_attendance(self, account, ceremony_index=None):
        cmd = ["get-proof-of-attendance", account]
        if ceremony_index is not None:
            cmd += ["--ceremony-index", str(ceremony_index)]
        ret = self.run_cli_command(cmd)
        return ret.stdout.decode("utf-8").strip()

    def set_meetup_time_offset(self, time_offset, cid=None, pay_fees_in_cc=False):
        cmd = ["set-meetup-time-offset", "--time-offset", str(time_offset)]
        ret = self.run_cli_command(cmd, cid=cid, pay_fees_in_cc=pay_fees_in_cc)
        ensure_clean_exit(ret)
        return ret.stdout.decode("utf-8").strip()
