**Reference workflow:** a single registered agent ("reference-agent"), granted
a Delegation by a human operator, executing tool calls against Agent Lab's
seeded demo data (`cust_1` owned by the reference operator; `cust_2` owned by
a different principal in the seed data and used here only as an out-of-scope
resource id, never actually read).

**Stated task need for this workflow (as would appear in a real engagement's
scope-of-work document):** the agent needs to read one customer's profile
(`get_customer` on `cust_1`) and, with human approval, add a case note for
that same customer. Nothing else.

**Actual grant issued (see AD-02):** the delegation used in this dry run
grants `get_customer`, `get_order`, and `add_case_note` against both `cust_1`
and `cust_2` — broader than the stated task need, on purpose, to produce a
realistic over-provisioning finding.

A second, narrower chained delegation (Agent A -> Agent B) is also issued to
demonstrate multi-hop scope attenuation (AD-03).
