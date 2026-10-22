import assert from "node:assert/strict";
import { test } from "node:test";
import { explainFailure, explainSkip } from "./explain.js";

test("failures are explained in plain words with a next step", () => {
  assert.match(explainFailure("timed out after 30s").summary, /No answer within 30 seconds/);
  assert.equal(explainFailure("timed out after 30s").fix?.tab, "sources");
  assert.match(explainFailure('HTTP 401 Unauthorized: {"error":"bad key"}').summary, /did not accept the key/);
  assert.equal(explainFailure("HTTP 403 Forbidden").fix?.tab, "keys");
  assert.match(explainFailure("HTTP 429 Too Many Requests").summary, /Out of free requests/);
  assert.match(explainFailure("HTTP 404 Not Found").summary, /does not know that model/);
  assert.match(explainFailure("HTTP 503 Service Unavailable").summary, /had a problem \(503\)/);
  assert.match(explainFailure("error sending request for url (http://localhost:11434)").summary, /Could not reach/);
  assert.equal(explainFailure("something odd").summary, "It did not answer.");
  assert.equal(explainFailure("HTTP 429 slow down").detail, "HTTP 429 slow down", "the original text is kept");
});

test("skipped sources are explained and name their fix", () => {
  const key = explainSkip("skipping provider: groq: no key: set GROQ_API_KEY or run `pampas key set GROQ_API_KEY`");
  assert.equal(key?.provider, "groq");
  assert.equal(key?.plain.fix?.tab, "keys");
  const limit = explainSkip("skipping provider: freeai: daily limit of 30 requests reached (30 used today, UTC)");
  assert.match(limit?.plain.summary ?? "", /used up \(30\)/);
  assert.equal(limit?.plain.fix?.tab, "usage");
  const setting = explainSkip("skipping provider: cf: cf: base_url uses ${CLOUDFLARE_ACCOUNT_ID}, which is not set: set the environment variable");
  assert.match(setting?.plain.summary ?? "", /CLOUDFLARE_ACCOUNT_ID/);
  assert.equal(explainSkip("only groq answered; nothing to merge"), null);
});
