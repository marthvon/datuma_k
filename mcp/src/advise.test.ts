import assert from "node:assert/strict";
import { test } from "node:test";
import { adviseNgin } from "./advise.js";

test("use ngin when two platforms share contract-derived UI", () => {
  const result = adviseNgin("Generate Zod schemas and Pydantic models from Event fields", [
    "api_server",
    "web_frontend",
  ]);
  assert.equal(result.use_ngin, true);
  assert.ok(result.reasons.length > 0);
});

test("skip ngin for one-off routing glue", () => {
  const result = adviseNgin("Add Express routing and OAuth middleware", ["api_server"]);
  assert.equal(result.use_ngin, false);
  assert.ok(result.reasons.some((line) => /handwritten|sync/i.test(line)));
});

test("skip ngin for a single platform", () => {
  const result = adviseNgin("Add a validation schema for Event", ["web_frontend"]);
  assert.equal(result.use_ngin, false);
});
