import assert from "node:assert/strict";
import { test } from "node:test";
import { inferPatterns, parseKeywordsTable, type Catalog } from "./infer.js";

const catalog: Catalog = {
  ok: true,
  models: [
    {
      name: "Event",
      traits: ["Resource"],
      fields: [
        { name: "title", type: "text_type", attributes: [{ name: "required", args: [] }] },
        { name: "email", type: "string", attributes: [] },
        { name: "sku", type: "string", attributes: [] },
      ],
    },
    {
      name: "Venue",
      traits: ["Resource"],
      fields: [
        { name: "title", type: "text_type", attributes: [{ name: "required", args: [] }] },
        { name: "email", type: "string", attributes: [] },
        { name: "sku", type: "string", attributes: [] },
      ],
    },
    {
      name: "Status",
      traits: [],
      fields: [],
    },
    {
      name: "UserRecord",
      traits: [],
      fields: [
        { name: "phone", type: "string", attributes: [] },
        { name: "opt_note", type: "string", attributes: [] },
      ],
    },
  ],
  types: ["text_type", "string", "int_type"],
  traits: ["Resource"],
  attributes: ["required"],
  fields: ["title", "email", "sku", "phone", "opt_note"],
};

test("similar field sets and Record suffix get Data; empty model gets Enum", () => {
  const result = inferPatterns(catalog, []);
  assert.ok(result.trait_suggestions.some((item) => item.target === "Event" && item.suggest === "Data"));
  assert.ok(result.trait_suggestions.some((item) => item.target === "Venue" && item.suggest === "Data"));
  assert.ok(result.trait_suggestions.some((item) => item.target === "Status" && item.suggest === "Enum"));
  assert.ok(result.trait_suggestions.some((item) => item.target === "UserRecord" && item.suggest === "Data"));
});

test("email phone sku and optional names get attributes", () => {
  const result = inferPatterns(catalog, []);
  assert.ok(
    result.attribute_suggestions.some((item) => item.target === "Event.email" && item.suggest === "email"),
  );
  assert.ok(
    result.attribute_suggestions.some((item) => item.target === "UserRecord.phone" && item.suggest === "phone_no"),
  );
  assert.ok(
    result.attribute_suggestions.some((item) => item.target === "Event.sku" && item.suggest === "unique"),
  );
  assert.ok(
    result.attribute_suggestions.some(
      (item) => item.target === "UserRecord.opt_note" && item.suggest === "nullable",
    ),
  );
});

test("aliases become standard_renames without rewriting", () => {
  const result = inferPatterns(catalog, []);
  assert.ok(result.standard_renames.some((item) => item.target === "text_type" && item.suggest === "string"));
  assert.ok(result.standard_renames.some((item) => item.target === "int_type" && item.suggest === "i32"));
});

test("single-platform unique and Data are sync risks", () => {
  const keywords = parseKeywordsTable(`
| keyword | kind | description | purpose | platforms |
| --- | --- | --- | --- | --- |
| unique | attribute | uniqueness | identity | api_server |
| Data | trait | struct | shared shape | api_server |
| email | attribute | email | contact | api_server, web_frontend |
`);
  const withUnique: Catalog = {
    ...catalog,
    models: [
      {
        name: "Account",
        traits: ["Data"],
        fields: [{ name: "sku", type: "string", attributes: [{ name: "unique", args: [] }] }],
      },
    ],
  };
  const result = inferPatterns(withUnique, keywords);
  assert.ok(result.sync_risks.some((item) => item.target === "unique"));
  assert.ok(result.sync_risks.some((item) => item.target === "Data"));
  assert.ok(!result.sync_risks.some((item) => item.target === "email"));
});

test("relationship and status get widgets; title does not get Text", () => {
  const withRel: Catalog = {
    ok: true,
    models: [
      {
        name: "Venue",
        traits: ["Data"],
        fields: [{ name: "title", type: "string", attributes: [{ name: "required", args: [] }] }],
      },
      {
        name: "Event",
        traits: ["Data"],
        fields: [
          { name: "title", type: "string", attributes: [{ name: "required", args: [] }] },
          { name: "status", type: "string", attributes: [] },
          { name: "published", type: "boolean", attributes: [] },
          { name: "venue", type: "relationship", attributes: [{ name: "required", args: [] }] },
        ],
      },
    ],
  };
  const result = inferPatterns(withRel, []);
  assert.ok(result.attribute_suggestions.some((item) => item.target === "Event.venue" && item.suggest === "Select"));
  assert.ok(result.attribute_suggestions.some((item) => item.target === "Event.venue" && item.suggest === "BelongsTo"));
  assert.ok(result.attribute_suggestions.some((item) => item.target === "Event.venue" && item.suggest === "Full"));
  assert.ok(result.attribute_suggestions.some((item) => item.target === "Event.venue" && item.suggest === "model"));
  assert.ok(result.attribute_suggestions.some((item) => item.target === "Event.status" && item.suggest === "Select"));
  assert.ok(result.attribute_suggestions.some((item) => item.target === "Event.published" && item.suggest === "Checkbox"));
  assert.ok(!result.attribute_suggestions.some((item) => item.target === "Event.title" && item.suggest === "Text"));
  assert.ok(!result.attribute_suggestions.some((item) => item.target === "Event.status" && item.suggest === "Text"));
});

test("string with Select is not a clash; stacked cardinality is an association suggestion", () => {
  const stacked: Catalog = {
    ok: true,
    models: [
      {
        name: "Event",
        traits: ["Data"],
        fields: [
          { name: "status", type: "string", attributes: [{ name: "Select", args: [] }] },
          {
            name: "tags",
            type: "relationship",
            attributes: [
              { name: "model", args: ["Tag"] },
              { name: "OneToOne", args: [] },
              { name: "ManyToMany", args: [] },
            ],
          },
        ],
      },
    ],
  };
  const result = inferPatterns(stacked, []);
  assert.ok(!result.association_suggestions.some((item) => item.target === "Event.status"));
  assert.ok(
    result.association_suggestions.some(
      (item) => item.target === "Event.tags" && item.suggest === "keep a single cardinality flag",
    ),
  );
  assert.ok(!result.attribute_suggestions.some((item) => item.target === "Event.status" && item.suggest === "Select"));
});
