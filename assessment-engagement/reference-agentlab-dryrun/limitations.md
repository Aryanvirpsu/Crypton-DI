- This is a **self-assessment of Crypton's own reference implementation**,
  not an independent assessment of a customer's system. Findings here
  demonstrate the *methodology and reporting mechanics*, not Crypton's
  differentiation versus any competitor or category.
- No external identity/HR/IT provisioning system exists for this synthetic
  engagement — AD-01 is UNVERIFIABLE for exactly that reason (evidence could
  not be obtained within this engagement's scope), not because the
  underlying concept is unrepresentable, and not because the question is
  unimportant.
- No live/production traffic exists for this engagement — AD-07 is
  NOT_APPLICABLE for the same reason.
- Agent Lab's WebAuthn step-up is a documented simulation
  (`webauthn.FakeWebAuthnVerifier`) — AD-06's approval-binding evidence
  demonstrates the *protocol design* (argument-hash binding, one-shot
  consumption), not a live authenticator ceremony.
- All findings trace to real, reproducible primitive calls made by
  `agentlab.assessment.reference_dryrun.build_reference_engagement` —
  re-running it against a fresh database reproduces the same
  ENFORCED/VIOLATED/UNVERIFIABLE/NOT_APPLICABLE pattern *and* the same
  evidence IDs byte-for-byte (delegation_id, decision_id, chain_hash, etc.
  are deterministic within this synthetic fixture — see the module
  docstring's "Determinism" note); a real customer engagement's evidence
  IDs are ordinary random values, not fixed like this reference fixture's.
- This engagement predates the 2026-09-30 schema expansion for most
  structured metadata fields (`target_commit`, `testing_window`, etc.) —
  those are set where genuinely known (this *is* Agent Lab, so its own
  product/version/commit are knowable) and left `None` where they
  describe a concept this synthetic, single-run engagement doesn't have
  (e.g. no multi-day `testing_window` — it is one scripted run).
