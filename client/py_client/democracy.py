import re
from datetime import datetime


class Proposal:
    def __init__(self, id, action, started, ends, start_cindex, electorate, turnout, approval, state):
        self.id = id
        self.action = action
        self.started = started
        self.ends = ends
        self.start_cindex = start_cindex
        self.electorate = electorate
        self.turnout = turnout
        self.approval = approval
        self.state = state


def parse_proposals(text):
    proposals = text.split("Proposal id:")
    proposal_objects = []

    for proposal in proposals[1:]:  # Skip the first split result as it will be an empty string
        proposal = "Proposal id:" + proposal  # Add back the identifier
        lines = proposal.split("\n")
        id = int(re.search(r'\d+', lines[0]).group())
        action = re.search(r'action:\w+\([\w, .]+\)', lines[1]).group()
        started = datetime.strptime(re.search(r'\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}', lines[2]).group(),
                                    '%Y-%m-%d %H:%M:%S')
        ends = datetime.strptime(re.search(r'\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}', lines[3]).group(),
                                 '%Y-%m-%d %H:%M:%S')
        start_cindex = int(re.search(r'\d+', lines[4]).group())
        electorate = int(re.search(r'\d+', lines[5]).group())
        turnout = int(re.search(r'\d+', lines[6]).group())
        approval = int(re.search(r'\d+', lines[7]).group())
        state = re.search(r'ProposalState::(\w+)', lines[8]).group(1)

        proposal_objects.append(Proposal(id, action, started, ends, start_cindex, electorate, turnout, approval, state))

    return proposal_objects
