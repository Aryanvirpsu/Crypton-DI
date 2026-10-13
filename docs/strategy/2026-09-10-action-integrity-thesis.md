# Crypton — Finalized Directional Research Thesis

**Date:** 10 September 2026
**Status:** Finalized directional research thesis
**Scope:** 2026–2031 research and company-direction framework

> This is a dated reference artifact, not the canonical instruction file.
> [`CLAUDE.md`](../../CLAUDE.md) stays the source of truth for day-to-day
> work — its §6–13 were rewritten around this thesis on 2026-09-10 (see
> [`CHANGELOG.md`](CHANGELOG.md) for exactly what changed and why, and for
> the prior wedge language this one supersedes). Where this document and
> `CLAUDE.md` disagree, `CLAUDE.md` governs; update this file only by
> adding a new dated thesis, not by editing this one in place.

**Important:** This is not yet a validated product thesis. It defines
what Crypton should investigate, what the first wedge is, why that wedge
fits us, what would make it defensible, and what evidence would falsify
it.

## 1. Core Direction

Crypton's long-term research sphere is:

**Security of Autonomous Actions**

The central security problem is not simply agent identity,
authentication, or policy.

It is:

When autonomous agents are allowed to cause consequential real-world
effects, can we determine what they are effectively capable of, whether
the exact action executed remained inside the authority and intent
granted to them, and what actually happened if something goes wrong?

This direction is broader than the original Crypton identity thesis, but
narrower than generic "AI security."

## 2. Permanent Strategic Filter

A candidate problem is more attractive if:

The security need becomes stronger as agents become more capable,
autonomous, interconnected, and economically important.

This is a permanent decision rule.

Prefer problems where:

```text
agent capability ↑
agent autonomy ↑
agent access ↑
agent economic importance ↑
            ↓
security need ↑
```

Avoid making the company dependent on problems likely to disappear as
models and frameworks improve.

## 3. Three Strategic Layers

Crypton now operates across three distinct levels.

**Long-Term Category**

Security of Autonomous Actions

The broad problem space.

**Medium-Term Architecture**

Four connected capabilities may eventually matter:

1. Effective Authority — What can an agent ultimately cause to happen?
2. Action / Intent Integrity — Did the exact executed action remain
   inside what was actually authorized?
3. Evidence / Forensics — Can the organization reconstruct and prove
   what happened?
4. Containment — Can compromised or malfunctioning agents be stopped
   without allowing failure to propagate?

Conceptually:

```text
                AUTONOMOUS ACTION
                       │
       ┌───────────────┼───────────────┐
       │               │               │
       ▼               ▼               ▼
  EFFECTIVE          ACTION          EVIDENCE
  AUTHORITY         INTEGRITY       / FORENSICS
       │               │               │
       └───────────────┼───────────────┘
                       ▼
                  CONTAINMENT
```

This is not one product we should attempt to build today.
It is the longer-term architecture we will investigate.

## 4. Near-Term Wedge

The first focused wedge is:

**Action Integrity Testing for Autonomous Security Agents**

The initial research and commercial question becomes:

Can an autonomous security agent take a consequential remediation or
enforcement action that exceeds, mutates, replays, inherits, or
continues beyond the authority or intent that originally justified it?

This is narrower than generic agent security.

It gives us:

- one class of agent;
- one buyer group;
- one failure category;
- one research question;
- one path to technical specialization.

## 5. Why Autonomous Security Agents First

Security agents are an unusually strong initial wedge because they may
receive high-impact permissions such as:

- disable user accounts;
- revoke sessions;
- rotate credentials;
- modify IAM;
- isolate endpoints;
- quarantine workloads;
- block network traffic;
- alter firewall rules;
- modify cloud configuration;
- patch software;
- change security policies;
- terminate workloads;
- perform incident-response actions.

That creates a simple risk:

A defensive agent may itself become a privileged attack or failure
surface.

The problem is not merely whether the agent possesses permission.
The deeper problem is whether its actual remediation effect remains
inside the authority that justified the action.

## 6. Core Failure Classes

Initial research should focus on concrete action-integrity failures.

**Scope Escalation**

Authorized:

```text
isolate endpoint X
```

Executed:

```text
isolate X, Y, and Z
```

**Argument Drift**

Authorized:

```text
disable user123
```

Executed:

```text
disable entire group
```

**Tool Substitution**

Authorized through one constrained tool.
Agent replans and performs an equivalent but broader action through
another interface.

**Semantic Replay**

A human authorizes a remediation once.
Retry/replanning produces a fresh execution that repeats the same
logical authorization.

**Delegation Escalation**

Parent agent delegates:

```text
investigate read-only
```

Child receives or discovers:

```text
containment / destructive privilege
```

**Revocation Failure**

Authority is revoked, but an already-running descendant continues to
act.

**Approval / Execution Drift**

Human approves one concrete action.
The action changes before execution without requiring fresh approval.

**Cross-Agent Confused Deputy**

One agent causes another more privileged agent to perform an action
outside the initiating agent's effective authority.

**Credential Inheritance**

A child/sub-agent inherits credentials whose power exceeds the
delegated task.

**Retry-Induced Overreach**

Error recovery or retries change the scope, arguments, target, or
number of executions.

## 7. First Buyer

The initial buyer is not:

Every company using AI agents.

The first buyer profile is:

Security engineering, security platform, SOC automation, or
security-product teams experimenting with agents capable of real
defensive actions.

Priority targets are teams where the phrase:

"Our agent can automatically…"

ends with something consequential.

Examples:

- isolate;
- revoke;
- rotate;
- block;
- patch;
- disable;
- terminate;
- reconfigure.

These teams already understand:

- blast radius;
- least privilege;
- approval;
- rollback;
- incident response.

That reduces the amount of market education required.

## 8. Initial Offer

Do not sell a platform.
Do not sell "agent identity."
Do not promise certification.

The initial offer is:

**Autonomous Security Agent Action-Integrity Assessment**

One narrow security-agent workflow.

We attempt to determine whether:

- delegated authority can widen;
- approvals can drift;
- actions can be replayed;
- retries alter scope;
- child agents inherit excessive authority;
- alternate tools bypass the intended boundary;
- revocation fails to propagate;
- actions cannot be reconstructed afterward.

Deliverables:

- authority-flow map;
- tested scenarios;
- reproducible findings;
- expected vs. observed behavior;
- affected action;
- consequence;
- evidence;
- remediation;
- retest.

## 9. Current Crypton Code Still Matters

Existing work is not discarded.

Current Agent Lab concepts include:

```text
agents.py
identity.py
delegation.py
chain.py
authorization.py
approval.py
tracing.py
```

These can serve as a controlled research laboratory for:

- principal identity;
- agent identity;
- delegated authority;
- authority attenuation;
- multi-hop chains;
- approval binding;
- pre-execution checks;
- revocation;
- trace/evidence generation.

Their role changes from:

Product infrastructure customers must adopt

to:

Reference implementation and experimental environment used to
understand and test security properties.

## 10. Technical Evolution

The immediate engineering goal is not a production platform.

The next useful evolution is a reusable adversarial testing methodology.

Potential future architecture:

```text
target workflow
      │
      ▼
 instrumentation
      │
      ▼
 test scenario
      │
      ▼
 authority/action mutation
      │
      ▼
 execution
      │
      ▼
 evidence capture
      │
      ▼
 security assertion
      │
      ▼
 PASS / FAIL / INCONCLUSIVE
```

Possible scenario families:

```text
scope escalation
delegation escalation
approval drift
semantic replay
revocation
tool substitution
credential inheritance
confused deputy
retry behavior
execution-boundary bypass
```

Build reusable tooling only after repeated evidence shows it saves real
assessment/research effort.

## 11. Why Us — Properly Framed

Crypton does not currently possess an unfair advantage.

It has a plausible right-to-win path.

That path comes from the combination of:

- existing Agent Lab implementation;
- hands-on work with identity, delegation, authorization and approval;
- offensive-security work;
- vulnerability-research experience;
- academic-security research access;
- ability to turn findings into reproducible technical research;
- ability to publish technical results before possessing major customer
  logos.

The strategic opportunity is to become known for novel autonomous-action
failure research, rather than attempting to look like an established
enterprise security consultancy.

## 12. Research Is Not a Side Activity

Research is one of the core engines of the strategy.

The desired loop is:

```text
experiment / assessment
        ↓
novel failure pattern
        ↓
reproducible attack technique
        ↓
technical write-up / publication
        ↓
credibility
        ↓
better collaborators / targets
        ↓
better experiments
        ↓
better methodology
```

Research can therefore provide:

- credibility;
- distribution;
- technical differentiation;
- customer discovery;
- methodology development.

Possible publication directions:

Authorization Drift in Autonomous Security Agents
Semantic Remediation Replay
Delegation Failures in Multi-Agent Security Workflows
When Approved Remediation Becomes Unauthorized Execution
Effective Authority in Autonomous Security Systems

The titles are illustrative. The contribution must come from genuine
experimental results.

## 13. The Moat Standard

We should no longer describe "having more assessment findings" as a moat
by itself.

A findings corpus becomes defensible only if:

1. patterns generalize;
2. those patterns materially improve future detection/testing;
3. the resulting knowledge is expensive to independently reproduce.

Therefore:

```text
10 customer engagements
≠ moat
```

Possible future defensibility could instead come from:

- novel attack classes;
- formal test models;
- widely recognized benchmarks;
- proprietary action-trace datasets;
- cross-framework test assertions;
- specialized instrumentation;
- framework adapters;
- research reputation;
- accumulated empirical failure models;
- detection/analysis engines trained by real evidence.

## 14. Hard Moat Test

At every major stage ask:

What do we possess that Microsoft, Palo Alto, CrowdStrike, Wiz, or
another well-funded security company could not reproduce within six
months by assigning a capable team?

Weak answer: We have experience.
Weak answer: We have ten customer reports.
Weak answer: We know agent authorization.

Potentially strong answers:
We discovered and formalized a new attack class.
We own a unique cross-framework dataset.
Our test engine detects a failure others do not model.
Our benchmark has become a standard way of evaluating the problem.
We possess difficult-to-recreate real-world action traces.
Researchers and security teams associate us with this technical
category.

If we cannot eventually produce a strong answer, we may have a good
service business but not a defensible software company.

## 15. Long-Term Expansion

If the initial wedge works, the product/research scope can expand
outward.

```text
ACTION INTEGRITY TESTING
          ↓
AUTHORITY MODELING
          ↓
EFFECTIVE AUTHORITY GRAPH
          ↓
CONTINUOUS ACTION MONITORING
          ↓
FORENSIC RECONSTRUCTION
          ↓
MULTI-AGENT CONTAINMENT
```

Potential future questions:

Effective Authority — If this agent is compromised, what can it
ultimately cause?
Action Integrity — Did the exact action remain within delegated
authority?
Evidence — Can we prove why this action happened?
Containment — Can we stop this agent and invalidate downstream
authority safely?

## 16. Long-Term Crypton Hypothesis

If successful, Crypton could eventually become a vendor-neutral security
layer around autonomous actions.

A future system might model:

```text
human principals
agents
sub-agents
credentials
delegations
tools
MCP servers
services
resources
memory provenance
approvals
executed actions
```

as a connected graph.

Then answer:

What can this agent cause?
Through which path?
Under whose authority?
What is the maximum blast radius?
Did execution remain inside authorized intent?
What changed during delegation or replanning?
What evidence proves the decision?
What needs to be revoked or quarantined after compromise?

This is directional architecture, not an immediate build specification.

## 17. 3–5 Year Horizon

Possible progression:

```text
2026
research + narrow assessments

        ↓

2027
repeatable action-integrity methodology
+ first design partners

        ↓

2028
test harness + structured failure corpus
+ authority/action modeling

        ↓

2029
continuous effective-authority /
action-integrity analysis

        ↓

2030
forensics + containment

        ↓

2031
cross-system autonomous-action
security infrastructure
```

This timeline is deliberately conditional.
Each stage must be unlocked by evidence from the previous stage.

## 18. Market-Timing Risk

The largest existential risk may not be technical.

It may be:

The market is too early.

If companies are not yet deploying security agents with meaningful
autonomous privileges, there may be nothing to sell despite the
long-term thesis being correct.

Therefore buyer validation is more important than continuing to
elaborate architecture.

The critical near-term questions are:

Are real teams giving agents consequential security permissions?
Who owns the problem?
Are they uncomfortable with the current controls?
Are existing vendors already solving it adequately?
Will they allow external testing?
Will anyone pay?

## 19. Explicit Kill Criteria

We must not spend three years saying:

The market is coming.

**By 4 November 2026**

We should have identified multiple credible teams actively building or
deploying agents with real security/remediation capabilities.

If almost none exist: commercial wedge is probably too early. Continue
research if technically valuable, but downgrade commercial expectations.

**By 15 December 2026**

Target: at least 5 serious technical conversations with teams working
on autonomous or semi-autonomous security actions.

If we cannot obtain these despite targeted outreach: investigate
whether buyer definition is wrong; wedge is too early; problem language
is wrong; access/credibility barrier is too high.

**By 31 January 2027**

We should have demonstrated at least one genuinely non-trivial
action-integrity failure in: our lab; an authorized open-source target;
or a collaborating external environment.

If all tests merely rediscover obvious IAM misconfiguration: technical
differentiation has not been demonstrated.

**By 31 March 2027**

At least one external team should be willing to collaborate on testing
a staging/non-production workflow.

If no one grants access after credible public work: the trust/access
problem may make the commercial wedge impractical.

**By 30 June 2027**

After: technical evidence; public research; a coherent methodology; at
least one design partner — we should test willingness to pay.

If nobody will purchase a scoped engagement: the commercial thesis is
weak even if the research remains interesting.

## 20. Validation Metrics

Do not use: commits; LOC; number of features; dashboard screens;
integrations; speculative architecture.

Track:

**Market** — qualified teams found; real autonomous actions identified;
buyer conversations; design partners; willingness to provide access;
willingness to pay.

**Technical** — meaningful failure classes discovered; reproducible
attacks; frameworks tested; false positives / negatives; generalization
across systems.

**Research** — novel hypotheses; experimental results; expert reviews;
responsible disclosures; technical publications; citations/adoption
where appropriate.

**Defensibility** — reusable assertions; unique datasets; specialized
instrumentation; repeated patterns; external adoption of methodology;
improvements driven by accumulated data.

## 21. What Remains Frozen

Do not use the new long-term vision as permission to return to
speculative engineering.

Remain HOLD unless evidence unlocks:

- proprietary universal agent identity;
- proprietary delegation protocol;
- cross-organization agent PKI;
- giant policy engine;
- huge dashboard;
- generic MCP-security platform;
- broad AI red-team suite;
- full authority graph product;
- full forensic SaaS;
- autonomous containment system;
- dozens of framework integrations.

## 22. Immediate Priorities

The next work should be:

1. Market discovery — Find actual security teams using agents with
   meaningful actions.
2. Literature / competitive research — Map current
   autonomous-security-agent products and research.
3. Threat model — Define action-integrity failure classes precisely.
4. Agent Lab experiments — Use the controlled environment to produce
   real failures and controls.
5. External authorized experimentation — Move into open-source or
   collaborator systems.
6. Technical publication — Turn valid discoveries into public evidence.
7. Design partner — Only after sufficient credibility.
8. Paid assessment — Only after demonstrated value.

## 23. What Would Change the Wedge

Action Integrity Testing for Autonomous Security Agents is the current
first wedge.

It is not sacred.

Change it if evidence shows that:

- autonomous security agents are not arriving soon enough;
- teams keep consequential actions human-gated;
- another vendor solves action integrity sufficiently;
- buyers care far more about forensic reconstruction;
- effective-authority analysis produces much stronger demand;
- a different research finding exposes a more urgent problem.

The long-term category can remain valid while the first wedge changes.

## 24. Final Strategic Position

Crypton should no longer ask:

How do we turn our existing identity technology into a product?

It should ask:

Which security failures become unavoidable when autonomous software
receives authority to cause consequential effects, and can we become
unusually good at finding and controlling one of them?

The current answer worth testing is:

Action Integrity Testing for Autonomous Security Agents

because it combines:

```text
real future risk
+
narrow initial buyer
+
technical novelty
+
existing Crypton knowledge
+
offensive-security fit
+
research potential
+
credible expansion path
```

The long-term thesis remains:

Security of Autonomous Actions

The possible future architecture is:

```text
EFFECTIVE AUTHORITY
        +
ACTION / INTENT INTEGRITY
        +
FORENSIC EVIDENCE
        +
CONTAINMENT
```

The current service/research wedge exists to determine whether any of
that deserves to become a company.

## 25. Operating Principle

Do not build the future company in advance.

Instead:

```text
FIND REAL USERS
      ↓
OBSERVE REAL ACTIONS
      ↓
DISCOVER REAL FAILURES
      ↓
FORMALIZE THE FAILURE
      ↓
PUBLISH / VALIDATE
      ↓
TEST REPEATABILITY
      ↓
EARN ACCESS
      ↓
EARN PAYMENT
      ↓
ACCUMULATE HARD-TO-REPLICATE KNOWLEDGE
      ↓
BUILD ONLY WHAT REPEATED EVIDENCE DEMANDS
```

**Final internal sentence**

Crypton is investigating the security of autonomous actions. Our first
wedge is action-integrity testing for autonomous security agents:
determining whether privileged defensive agents can exceed, mutate,
replay, inherit, or continue authority beyond the remediation humans or
upstream systems actually intended. We will use research, adversarial
testing, and real-world evidence to determine whether this becomes a
defensible security company or remains a valuable research direction.
