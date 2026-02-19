from py_client.base import _BaseClient, Error, ExtrinsicWrongPhase, ExtrinsicFeePaymentImpossible, \
    ParticipantAlreadyLinked, UnknownError, ensure_clean_exit
from py_client.ceremonies_mixin import _CeremonyMixin
from py_client.communities_mixin import _CommunityMixin
from py_client.balances_mixin import _BalanceMixin
from py_client.bazaar_mixin import _BazaarMixin
from py_client.faucet_mixin import _FaucetMixin
from py_client.democracy_mixin import _DemocracyMixin
from py_client.offline_payment_mixin import _OfflinePaymentMixin
from py_client.reputation_rings_mixin import _ReputationRingsMixin
from py_client.treasury_mixin import _TreasuryMixin
from py_client.reputation_commitments_mixin import _ReputationCommitmentsMixin
from py_client.ipfs_mixin import _IpfsMixin
from py_client.metadata_mixin import _MetadataMixin


class Client(
    _BaseClient,
    _CeremonyMixin,
    _CommunityMixin,
    _BalanceMixin,
    _BazaarMixin,
    _FaucetMixin,
    _DemocracyMixin,
    _OfflinePaymentMixin,
    _ReputationRingsMixin,
    _TreasuryMixin,
    _ReputationCommitmentsMixin,
    _IpfsMixin,
    _MetadataMixin,
):
    """Encointer CLI client. See individual mixins for method docs."""
    pass


# Re-export for backward compatibility
__all__ = [
    'Client',
    'Error',
    'ExtrinsicWrongPhase',
    'ExtrinsicFeePaymentImpossible',
    'ParticipantAlreadyLinked',
    'UnknownError',
    'ensure_clean_exit',
]
