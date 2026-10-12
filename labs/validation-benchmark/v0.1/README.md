# Crypton Validation Benchmark v0.1

**Purpose:** before asking any customer for access, show that Crypton's
methodology can distinguish a *known-vulnerable* implementation from its
own *patched* successor — using dependencies Crypton did not write and
vulnerabilities Crypton did not discover. This exists specifically to
break a circularity problem in Crypton's prior evidence: "we designed the
tests, we ran the tests, our tests passed" is not, by itself, evidence
that the tests measure anything real. Testing against an independently
disclosed, independently patched CVE removes that circularity — the
"ground truth" (vulnerable vs. fixed) comes from someone else's
disclosure and someone else's fix commit, not from Crypton's own
judgment.

**What this is not:** Crypton did not discover any vulnerability in this
directory. Every case here reproduces a fix that already shipped,
against a CVE that was already public, using the vendor's own before/after
versions. That is stated in every case's own `README.md` and is not
softened anywhere.

## Ideal shape, per case

```
known vulnerable version -> Crypton's test -> FAIL
patched version          -> the SAME, unmodified Crypton test -> PASS
```

The test script does not change between runs. Only the installed
dependency version changes. If a case can't be made to work this way — if
the test needs to be different for the two versions to produce the
"right" answer — it does not belong in this benchmark, because it would
no longer be evidence about the dependency, just evidence about the test
writer's expectations.

## Cases

| ID | Target | CVE / advisory | Family | Vendor / language | Result |
|---|---|---|---|---|---|
| **VB-001** | [Official MCP Python SDK](case-01-mcp-python-sdk-session-binding/) (`mcp` on PyPI), `StreamableHTTPSessionManager` | [CVE-2026-52869](https://github.com/advisories/GHSA-jpw9-pfvf-9f58) | Principal / Session Binding | `modelcontextprotocol` org, Python | **Reproduced.** `mcp==1.27.1`: FAIL. `mcp==1.27.2`: PASS. |
| **VB-002** | [`@bytebase/dbhub`](case-02-dbhub-sqlite-readonly-boundary/) (npm), SQLite connector | CVE-2026-61788 | Authorization Boundary & Complete Mediation | `bytebase`, TypeScript/Node | **Reproduced.** `0.22.5`: FAIL. `0.22.6`: PASS. |

Each case's own script is run completely unmodified between the
vulnerable and fixed runs — only the installed dependency version
changes. VB-001 and VB-002 are deliberately independent of each other:
different vendor, different language/runtime, different property family,
different underlying mechanism (a missing identity check vs. an
enforcement pragma that was never invoked). Two independent systems
answer a different question than one: not just "is this test
self-referential" (VB-001 alone already answers that) but "does the
methodology transfer to a system built by different people, in a
different language, testing a different property" — see
`pilot-pack/v0.1/07-why-trust-crypton.md`.

A third case (`mcp-atlassian`, CVE-2026-77243) was investigated and set
aside: the fix landed inside a larger refactor across several releases
rather than one isolable patch, and forcing a case through under time
pressure would have risked exactly the kind of rushed, less-trustworthy
evidence this benchmark exists to avoid. Two cases, done rigorously and
reproducibly, are worth more than three done under pressure.

## How to reproduce these yourself

```bash
# VB-001
cd labs/validation-benchmark/v0.1/case-01-mcp-python-sdk-session-binding
python -m venv .venv-vuln && .venv-vuln/Scripts/pip install mcp==1.27.1
python -m venv .venv-fixed && .venv-fixed/Scripts/pip install mcp==1.27.2
.venv-vuln/Scripts/python test_session_binding.py    # -> FAIL
.venv-fixed/Scripts/python test_session_binding.py   # -> PASS

# VB-002
cd ../case-02-dbhub-sqlite-readonly-boundary
mkdir vuln fixed
(cd vuln && npm init -y && npm install @bytebase/dbhub@0.22.5)
(cd fixed && npm init -y && npm install @bytebase/dbhub@0.22.6)
cp test_readonly_boundary.mjs vuln/ && cp test_readonly_boundary.mjs fixed/
node vuln/test_readonly_boundary.mjs     # -> FAIL
node fixed/test_readonly_boundary.mjs    # -> PASS
```

No network access, no real credentials, no external service for either
case — both probes drive the installed package's own public API directly,
in-process.

## What this proves, and what it doesn't

**Proves:** Crypton's tests for two properties (Principal/Session Binding,
Authorization Boundary & Complete Mediation) produce the correct verdict
against real dependencies' known-vulnerable and known-fixed states, using
unmodified scripts, across two unrelated vendors and languages.

**Does not prove:** that Crypton's methodology works against an arbitrary,
unfamiliar customer system — this benchmark is retrospective, against
documented fixes, which is a meaningfully easier problem than finding an
undocumented gap in a system nobody has looked at yet. It also doesn't
mean Crypton found anything here — the vendors found and fixed these
before Crypton ever touched them. See `pilot-pack/v0.1/04-evidence-map.md`
for how this is kept separate from a claim of discovery.

## Roadmap

A third, independent case (ideally covering the remaining two Core
families — Delegation & Scope Attenuation, or Approval-to-Execution
Binding) would round this out further, if one can be found with the same
"one isolable patch, safely locally reproducible" property VB-001 and
VB-002 both had. No case is added just to inflate the count.
