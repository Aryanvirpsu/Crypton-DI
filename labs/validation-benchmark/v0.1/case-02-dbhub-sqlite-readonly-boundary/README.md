# Case 02 (VB-002) — Authorization Boundary vs. `@bytebase/dbhub`'s SQLite connector

**Target:** `@bytebase/dbhub` (npm), a database MCP server — the SQLite
connector's `executeSQL()`. **Advisory:** CVE-2026-61788, "Incorrect
Authorization," CVSS 7.4. **Vulnerable:** `< 0.22.6` (tested at `0.22.5`).
**Fixed:** `>= 0.22.6` (tested at `0.22.6`). **Crypton test-catalog
family:** Authorization Boundary & Complete Mediation
(`pilot-pack/v0.1/03-test-catalog.md`, Family 3).

**Why this is a genuinely independent second case, not a repeat of
VB-001:** different vendor (`bytebase`, unrelated to the MCP SDK's
maintainers), different language and runtime (TypeScript/Node vs.
Python), different property family (a claimed execution-scope boundary
that was never wired to anything, vs. a session/principal-binding gap),
and a different underlying mechanism (a SQL-engine enforcement pragma
that was simply never invoked, vs. a missing identity comparison).

## What the advisory says

The public advisory (originally reported against the PostgreSQL
connector) describes setting `readonly = true` on the SQL-execution tool
as not actually making the connection read-only: the code intended to
enforce it existed, but the config value gating it was never populated,
so it never ran. Reading the actual patch between `0.22.5` and `0.22.6`
(this session, from the published npm tarballs), the same root pattern —
a `readonly` option accepted and threaded through the code, but never
translated into an actual database-level restriction — is present, and
separately fixed, in **every** connector in this release, including
SQLite: the vulnerable `SQLiteConnector.executeSQL()` never issues
`PRAGMA query_only = ON`, so a caller that requests `{ readonly: true }`
gets a connection that can still execute arbitrary writes. `0.22.6` adds
the pragma call before executing, and resets it afterward.

## What Crypton's test does

`test_readonly_boundary.mjs`, run unmodified against both versions:

1. Locates the installed package's own SQLite connector module (a
   content-hashed chunk file whose name changes every build, found by
   pattern rather than a hardcoded path) and imports the exported
   `SQLiteConnector` class directly — no MCP transport, no network, no
   server process.
2. Opens an in-memory database and seeds two rows.
3. **Negative control, run first, on row 2:** an ordinary write
   (`readonly` omitted) must actually change the row. If it doesn't, the
   connector's write path itself is broken here and the probe below would
   be meaningless — the script checks this and exits `1` without
   reporting a verdict if it happens. It did not happen on either
   version.
4. **The probe, on row 1:** a write attempted with `{ readonly: true }`.
   Records whether it throws, and whether the row actually changed.
5. If the row changed despite `readonly: true` → **FAIL**. If the write
   was rejected (SQLite's own `SQLITE_READONLY` error, via the
   `query_only` pragma) and the row is unchanged → **PASS**.

## Actual results (this session)

```
$ node test_readonly_boundary.mjs   # @bytebase/dbhub@0.22.5
Negative control (ordinary UPDATE on row 2 actually wrote "control-mutated"): ok
Readonly-write attempt threw an error: false
Row 1 value before: original-a / after: readonly-mutated
VERDICT: FAIL -- VULNERABLE (a write executed under { readonly: true } and
the row actually changed; the readonly claim was never enforced)

$ node test_readonly_boundary.mjs   # @bytebase/dbhub@0.22.6
Negative control (ordinary UPDATE on row 2 actually wrote "control-mutated"): ok
Readonly-write attempt threw an error: true
Row 1 value before: original-a / after: original-a
VERDICT: PASS -- SECURE (the write under { readonly: true } was rejected
by the database itself, and the row is unchanged)
  note: readonly write attempt raised: attempt to write a readonly database
```

Full JSON evidence: [`evidence/vulnerable_0.22.5.json`](evidence/vulnerable_0.22.5.json),
[`evidence/fixed_0.22.6.json`](evidence/fixed_0.22.6.json). Exit code was
`0` on both runs (negative control held on both versions).

## Reproduce it yourself

```bash
mkdir vuln fixed && cd vuln && npm init -y && npm install @bytebase/dbhub@0.22.5 && cd ..
cd fixed && npm init -y && npm install @bytebase/dbhub@0.22.6 && cd ..
cp test_readonly_boundary.mjs vuln/ && cp test_readonly_boundary.mjs fixed/
node vuln/test_readonly_boundary.mjs     # -> FAIL
node fixed/test_readonly_boundary.mjs    # -> PASS
```

Requires Node >= 22.5 (uses the built-in `node:sqlite` module, which the
connector itself depends on).

## Independence from the vendor's own fix

Written by reading the public advisory plus the published npm tarballs'
compiled output for both versions directly (this project's source is not
included unminified in the npm package) — not the vendor's own test
suite, which was not consulted.

## Limitations

- The public advisory's own description centers the PostgreSQL connector;
  this case tests the SQLite connector specifically, which this session's
  own diff of the published package confirmed has the identical
  vulnerable-then-fixed pattern, but that confirmation is Crypton's own
  reading of the diff, not a restatement of the advisory's own text.
- Reproduction reads compiled/bundled output (`dist/*.js`), not the
  project's original TypeScript source, because the source is not part of
  the published npm package — this is what an assessor working from a
  customer's installed dependency (rather than its source repository)
  would also be limited to.
- Only the SQLite connector's in-memory mode is exercised. The PostgreSQL,
  MySQL, SQL Server, and MariaDB connectors are not retested here.
