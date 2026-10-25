"""
WebAuthn boundary for human approval (v0.5).

Real integration point: crypton-id already owns passkey registration and
authentication end to end (services/crypton-id/src/routes/webauthn.rs,
.../jwt.rs) — the same WebAuthn ceremony `identity.verify_human_identity`
already trusts for login. Agent Lab does not reimplement WebAuthn, and
this module does not perform a live ceremony itself. It defines the
*interface* a step-up verification must satisfy, and `approval.py`
depends only on that interface — the same pattern `model_provider.py`
already uses to keep the agent loop independent of any one LLM vendor.

What's real vs. simulated, explicitly:
  - REAL: the *shape* of the security property — a human must complete a
    WebAuthn ceremony, immediately before an Approval can move
    pending -> approved, and the ceremony's challenge is bound to the
    exact approval being granted (see `approve()` in approval.py, which
    derives the challenge from `approval_id` + `argument_hash` rather
    than accepting a caller-supplied one). This is how a real integration
    would have to be wired, not just how the fake happens to work.
  - SIMULATED (Lab-only): `FakeWebAuthnVerifier` never talks to an
    authenticator or to crypton-id. It deterministically manufactures
    `WebAuthnEvidence` so the Lab's tests and `--fake` CLI flow can
    exercise the full approval pipeline without a live passkey. Every
    piece of evidence it produces sets `is_simulated=True` and is
    prefixed `fake:` — never mistakeable for a real assertion reference.
    Wiring a `CryptonIdWebAuthnVerifier` that actually calls crypton-id's
    step-up endpoint is future integration work; this interface is what
    makes that a pluggable swap instead of a redesign.
"""

from __future__ import annotations

import hashlib
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass

from .identity import HumanIdentity


class WebAuthnVerificationError(Exception):
    """Raised when step-up verification fails or is refused. The approval
    this was for is never transitioned — it stays exactly as pending as
    it was before the attempt."""


@dataclass(frozen=True)
class WebAuthnEvidence:
    """Opaque reference to one completed step-up ceremony.

    `reference` is what `Approval.webauthn_reference` persists — in a
    real integration this would be crypton-id's credential/assertion id,
    never raw assertion bytes, a private key, or any other secret
    material. `is_simulated` is never omitted or defaulted away: any code
    that treats a `WebAuthnEvidence` as proof of a real ceremony without
    checking it is a bug.
    """

    credential_id: str
    verified_at: float
    reference: str
    is_simulated: bool


class WebAuthnVerifier(ABC):
    """Implement this to back approval step-up with a real ceremony. The
    approval pipeline depends only on this interface, never on a
    concrete verifier — see model_provider.ModelProvider for the same
    pattern applied to LLM vendors."""

    @abstractmethod
    def verify_step_up(self, *, principal: HumanIdentity, challenge: str) -> WebAuthnEvidence:
        """Verify a step-up WebAuthn ceremony for `principal`, bound to
        `challenge`. Must raise WebAuthnVerificationError rather than
        returning anything on failure — there is no "invalid but still an
        evidence object" return value."""
        ...


class FakeWebAuthnVerifier(WebAuthnVerifier):
    """Deterministic Lab/test-only simulation — see module docstring.
    Never a real ceremony, never used outside Agent Lab's own tests and
    `--fake` CLI flow.

    `FAIL_CHALLENGE` is a sentinel: passing it as the challenge always
    raises, so tests can exercise WEBAUTHN_VERIFICATION_FAILED without a
    real authenticator that could actually refuse."""

    FAIL_CHALLENGE = "__force_webauthn_failure__"

    def verify_step_up(self, *, principal: HumanIdentity, challenge: str) -> WebAuthnEvidence:
        if challenge == self.FAIL_CHALLENGE:
            raise WebAuthnVerificationError("simulated step-up failure")
        digest = hashlib.sha256(f"{principal.user_id}:{principal.cred_id}:{challenge}".encode("utf-8")).hexdigest()
        return WebAuthnEvidence(
            credential_id=principal.cred_id,
            verified_at=time.time(),
            reference=f"fake:{digest}",
            is_simulated=True,
        )
