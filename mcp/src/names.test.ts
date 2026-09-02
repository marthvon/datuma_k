import assert from "node:assert/strict";
import { test } from "node:test";
import { TOOL_NAMES } from "./names.js";

test("registers the compiler tools plus advise_ngin", () => {
  assert.deepEqual([...TOOL_NAMES], [
    "list_project",
    "query_contracts",
    "validate",
    "preview",
    "generate",
    "advise_ngin",
  ]);
});
