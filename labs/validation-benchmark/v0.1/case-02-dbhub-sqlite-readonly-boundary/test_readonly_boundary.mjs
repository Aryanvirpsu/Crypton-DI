/**
 * Crypton Validation Benchmark v0.1 -- Case 02 (VB-002)
 * Authorization Boundary vs. @bytebase/dbhub's SQLite connector
 *
 * Independently written by Crypton against the installed `@bytebase/dbhub`
 * package's SQLite connector (`SQLiteConnector.executeSQL`). Does not import
 * or reuse the project's own test suite -- this is a separately authored
 * probe, run unmodified against two different installed versions of the
 * same dependency, so any difference in outcome is attributable to the
 * dependency version, not to the test.
 *
 * Maps to Crypton test-catalog Family 3 (Authorization Boundary & Complete
 * Mediation, `pilot-pack/v0.1/03-test-catalog.md`) and the external case
 * documented in `pilot-pack/v0.1/04-evidence-map.md`: CVE-2026-61788.
 * Vulnerable: @bytebase/dbhub < 0.22.6 (tested at 0.22.5). Fixed: >= 0.22.6
 * (tested at 0.22.6).
 *
 * The advisory: setting `readonly: true` on the execute-SQL tool claims to
 * make the connection read-only, but the code path that was supposed to
 * enforce it for the SQLite connector never actually ran on the vulnerable
 * version -- a claimed boundary that was never wired to anything.
 *
 * What this script does, deterministically, with no network, no external
 * database server, and no real credentials:
 *   1. Locates the installed package's own SQLite connector module. The
 *      module lives in a content-hashed chunk file (`dist/sqlite-*.js`)
 *      that changes name every build, so this script resolves it by
 *      pattern from the installed package's own directory rather than a
 *      hardcoded path -- the only way to reach it stably across versions.
 *   2. Opens an in-memory SQLite database (`:memory:`) through the
 *      connector's own public `connect()`/`executeSQL()` API and seeds one
 *      row.
 *   3. Calls `executeSQL()` with `{ readonly: true }` on a plain UPDATE
 *      statement against that row -- exactly the call an MCP client makes
 *      when a customer has configured this tool as read-only.
 *   4. Checks whether the row actually changed. It should not.
 *   5. Runs a negative control: the SAME UPDATE, with `{ readonly: false }`,
 *      against a second row, MUST succeed and actually change data --
 *      otherwise "the readonly row didn't change" would just mean the
 *      connector is broken for writes generally, not that readonly is
 *      correctly enforced.
 *
 * Run with the version under test already installed in `node_modules`:
 *     node test_readonly_boundary.mjs [--json]
 *
 * Exit code 0 on a clean, determinate result either way (VULNERABLE or
 * SECURE) with the negative control intact; exit code 1 if the negative
 * control itself fails.
 */

import { createRequire } from "node:module";
import { readdirSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const require_ = createRequire(import.meta.url);

function installedVersion() {
  try {
    return require_("@bytebase/dbhub/package.json").version;
  } catch {
    return "unknown";
  }
}

async function loadSQLiteConnectorClass() {
  // dist/index.js is the package's declared "main" -- its directory also
  // holds the content-hashed connector chunks. The SQLite chunk file name
  // (e.g. "sqlite-2GBIF3MQ.js") is not a stable public path, so it is found
  // by pattern within the installed package's own dist directory.
  const mainEntry = require_.resolve("@bytebase/dbhub");
  const distDir = path.dirname(mainEntry);
  const chunkName = readdirSync(distDir).find((f) => /^sqlite-.*\.js$/.test(f));
  if (!chunkName) {
    throw new Error(`could not find a sqlite-*.js chunk under ${distDir}`);
  }
  const mod = await import(pathToFileURL(path.join(distDir, chunkName)).href);
  return mod.SQLiteConnector;
}

async function freshConnector(SQLiteConnector) {
  const connector = new SQLiteConnector();
  await connector.connect(
    "sqlite:///:memory:",
    "CREATE TABLE crypton_probe (id INTEGER PRIMARY KEY, v TEXT NOT NULL);" +
      "INSERT INTO crypton_probe (id, v) VALUES (1, 'original-a'), (2, 'original-b');"
  );
  return connector;
}

async function readValue(connector, id) {
  const { rows } = await connector.executeSQL(`SELECT v FROM crypton_probe WHERE id = ${id}`, {
    readonly: true,
  });
  return rows[0]?.v;
}

async function attemptUpdate(connector, id, newValue, readonly) {
  try {
    await connector.executeSQL(`UPDATE crypton_probe SET v = '${newValue}' WHERE id = ${id}`, {
      readonly,
    });
    return { threw: false, error: null };
  } catch (error) {
    return { threw: true, error: String(error?.message ?? error) };
  }
}

async function runProbe() {
  const result = {
    installed_version: installedVersion(),
    negative_control_ok: false,
    negative_control_value_after: null,
    readonly_attempt_threw: null,
    readonly_value_before: null,
    readonly_value_after: null,
    verdict: "INCONCLUSIVE",
    notes: [],
  };

  const SQLiteConnector = await loadSQLiteConnectorClass();
  const connector = await freshConnector(SQLiteConnector);

  // Negative control FIRST, on a different row: an ordinary, non-readonly
  // UPDATE must actually change data on both versions under test. If it
  // doesn't, this probe cannot tell "readonly correctly blocked a write"
  // apart from "writes are broken here entirely."
  await attemptUpdate(connector, 2, "control-mutated", false);
  result.negative_control_value_after = await readValue(connector, 2);
  result.negative_control_ok = result.negative_control_value_after === "control-mutated";

  if (!result.negative_control_ok) {
    result.verdict = "INCONCLUSIVE -- negative control failed, see notes";
    result.notes.push(
      `an ordinary (non-readonly) UPDATE did not change row 2 (value is ` +
        `"${result.negative_control_value_after}"); the connector's own write ` +
        "path is not behaving as expected here, so the readonly probe below cannot be trusted."
    );
    await connector.disconnect();
    return result;
  }

  // The actual probe: row 1, readonly: true, a write attempt.
  result.readonly_value_before = await readValue(connector, 1);
  const attempt = await attemptUpdate(connector, 1, "readonly-mutated", true);
  result.readonly_attempt_threw = attempt.threw;
  if (attempt.threw) {
    result.notes.push(`readonly write attempt raised: ${attempt.error}`);
  }
  result.readonly_value_after = await readValue(connector, 1);

  const wasMutated = result.readonly_value_after !== result.readonly_value_before;

  if (wasMutated) {
    result.verdict =
      "FAIL -- VULNERABLE (a write executed under { readonly: true } and the row actually changed; " +
      "the readonly claim was never enforced)";
  } else if (attempt.threw) {
    result.verdict =
      "PASS -- SECURE (the write under { readonly: true } was rejected by the database itself, " +
      "and the row is unchanged)";
  } else {
    result.verdict =
      "INCONCLUSIVE -- the write did not throw and the row did not change; " +
      "neither a clean rejection nor a clean bypass was observed";
  }

  await connector.disconnect();
  return result;
}

async function main() {
  const asJson = process.argv.includes("--json");
  const result = await runProbe();

  if (asJson) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log("Crypton Validation Benchmark v0.1 -- Case 02: Authorization Boundary (readonly claim)");
    console.log(`Installed @bytebase/dbhub version: ${result.installed_version}`);
    console.log(
      `Negative control (ordinary UPDATE on row 2 actually wrote "control-mutated"): ${
        result.negative_control_ok ? "ok" : "FAILED -- see notes"
      }`
    );
    console.log(`Readonly-write attempt threw an error: ${result.readonly_attempt_threw}`);
    console.log(`Row 1 value before: ${result.readonly_value_before} / after: ${result.readonly_value_after}`);
    console.log(`VERDICT: ${result.verdict}`);
    for (const note of result.notes) {
      console.log(`  note: ${note}`);
    }
  }

  process.exit(result.negative_control_ok ? 0 : 1);
}

main();
