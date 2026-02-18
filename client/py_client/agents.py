from enum import Enum
from dataclasses import dataclass, field


class AgentRole(Enum):
    BOOTSTRAPPER = "bootstrapper"
    REPUTABLE = "reputable"
    NEWBIE = "newbie"
    MERCHANT = "merchant"


@dataclass
class Agent:
    account: str
    role: AgentRole
    ceremony_count: int = 0
    has_business: bool = False
    has_offline_identity: bool = False
    bandersnatch_key: str | None = None

    @property
    def is_bootstrapper(self):
        return self.role == AgentRole.BOOTSTRAPPER

    @property
    def is_reputable(self):
        return self.role in (AgentRole.REPUTABLE, AgentRole.BOOTSTRAPPER, AgentRole.MERCHANT)

    @property
    def can_endorse(self):
        return self.role == AgentRole.BOOTSTRAPPER

    def promote(self):
        """Promote newbie to reputable after attending a ceremony."""
        if self.role == AgentRole.NEWBIE:
            self.role = AgentRole.REPUTABLE

    @property
    def has_bandersnatch(self):
        return self.bandersnatch_key is not None
