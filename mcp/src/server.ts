import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { McpServer } from "@modelcontextprotocol/server";
import * as z from "zod/v4";
import { adviseNgin } from "./advise.js";
import { jsonToolResult, runDatumaK } from "./cli.js";
import { inferPatterns, parseKeywordsTable, type Catalog } from "./infer.js";
import { listProject } from "./listProject.js";

const ROOT = z.object({
  root: z.string().optional().describe("Project directory (defaults to the MCP process cwd)"),
});

const PLATFORMS = z.enum(["api_server", "web_frontend", "mobile_frontend"]);

const RESOURCES: { name: string; uri: string; title: string; file: string }[] = [
  {
    name: "dtct",
    uri: "datuma://language/dtct",
    title: "How to write .dtct",
    file: "dtct.md",
  },
  {
    name: "ngin",
    uri: "datuma://language/ngin",
    title: "How to write .ngin",
    file: "ngin.md",
  },
  {
    name: "keywords",
    uri: "datuma://docs/keywords",
    title: "Keyword documentation rules",
    file: "keywords.md",
  },
  {
    name: "practices",
    uri: "datuma://docs/practices",
    title: "datuma_k good practices",
    file: "practices.md",
  },
  {
    name: "when-ngin",
    uri: "datuma://docs/when-ngin",
    title: "When to use ngin",
    file: "when-ngin.md",
  },
  {
    name: "standards",
    uri: "datuma://language/standards",
    title: "Standard dtct vocabulary",
    file: "standards.md",
  },
];

export function createDatumaServer(): McpServer {
  const server = new McpServer({ name: "datuma_k", version: "1.0.0" });
  const resourcesDir = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "resources");

  server.registerTool(
    "list_project",
    {
      title: "List datuma_k project",
      description:
        "List .env directories and *.dtct, *.ngin, *.def.ngin, and keywords.md for a datuma_k project.",
      inputSchema: ROOT,
      annotations: { readOnlyHint: true, destructiveHint: false },
    },
    async ({ root }) => {
      const listed = await listProject(root ?? process.cwd());
      const text = JSON.stringify(listed, null, 2);
      return { content: [{ type: "text", text }] };
    },
  );

  server.registerTool(
    "query_contracts",
    {
      title: "Query contracts",
      description:
        "JSON catalog of dtct models, fields, traits, types, and attributes. Optional filters match dk.trait / dk.model / dk.field / dk.attribute / dk.type.",
      inputSchema: ROOT.extend({
        trait: z.string().optional(),
        model: z.string().optional(),
        field: z.string().optional(),
        attribute: z.string().optional(),
        type: z.string().optional(),
      }),
      annotations: { readOnlyHint: true, destructiveHint: false },
    },
    async ({ root, trait, model, field, attribute, type }) => {
      const args = ["catalog"];
      const filters: [string, string | undefined][] = [
        ["--trait", trait],
        ["--model", model],
        ["--field", field],
        ["--attribute", attribute],
        ["--type", type],
      ];
      for (const [flag, value] of filters) {
        if (value) {
          args.push(flag, value);
        }
      }
      const result = await runDatumaK(args, root ?? process.cwd());
      return jsonToolResult(result.stdout, result.stderr, result.status);
    },
  );

  server.registerTool(
    "validate",
    {
      title: "Validate project",
      description:
        "Parse *.dtct / *.ngin / *.def.ngin and require a data/keywords.md table (kind, description, purpose, platforms) for every contract name. Does not write files.",
      inputSchema: ROOT,
      annotations: { readOnlyHint: true, destructiveHint: false },
    },
    async ({ root }) => {
      const result = await runDatumaK(["check"], root ?? process.cwd());
      return jsonToolResult(result.stdout, result.stderr, result.status);
    },
  );

  server.registerTool(
    "preview",
    {
      title: "Preview generation",
      description:
        "Plan generated files from ngin without committing. Returns planned paths and content. Does not write disk.",
      inputSchema: ROOT,
      annotations: { readOnlyHint: true, destructiveHint: false },
    },
    async ({ root }) => {
      const result = await runDatumaK(["preview"], root ?? process.cwd());
      return jsonToolResult(result.stdout, result.stderr, result.status);
    },
  );

  server.registerTool(
    "generate",
    {
      title: "Generate files",
      description:
        "Run datuma_k run: commit planned files through dkcache. Overwrites text inside generated spans; keeps text between spans.",
      inputSchema: ROOT,
      annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false },
    },
    async ({ root }) => {
      const result = await runDatumaK(["run"], root ?? process.cwd());
      if (result.status === 0) {
        return { content: [{ type: "text", text: result.stdout.trim() || "generated" }] };
      } else {
        return {
          content: [
            {
              type: "text",
              text: result.stderr.trim() || result.stdout.trim() || `datuma_k run exited ${result.status}`,
            },
          ],
          isError: true,
        };
      }
    },
  );

  server.registerTool(
    "advise_ngin",
    {
      title: "Advise ngin vs handwritten",
      description:
        "Decide whether a coding task should use the ngin template engine or stay handwritten. Use ngin when two or more platforms must stay in sync on contract-derived types, validation, or UI.",
      inputSchema: z.object({
        task: z.string().describe("What the agent wants to add or generate"),
        platforms: z
          .array(PLATFORMS)
          .describe("Where the result will ship"),
      }),
      annotations: { readOnlyHint: true, destructiveHint: false },
    },
    async ({ task, platforms }) => {
      const advice = adviseNgin(task, platforms);
      return { content: [{ type: "text", text: JSON.stringify(advice, null, 2) }] };
    },
  );

  server.registerTool(
    "infer_patterns",
    {
      title: "Infer traits and attributes",
      description:
        "Suggest shared Data/Enum traits, field attributes, and UI widgets from the contract. Flag unusual default-vocabulary pairings as association_suggestions (review notes, not errors). Flag single-platform sync risks. Map aliases like text_type to string. Does not write files or reject contracts.",
      inputSchema: ROOT,
      annotations: { readOnlyHint: true, destructiveHint: false },
    },
    async ({ root }) => {
      const cwd = root ?? process.cwd();
      const listed = await listProject(cwd);
      const result = await runDatumaK(["catalog"], cwd);
      const failed = jsonToolResult(result.stdout, result.stderr, result.status);
      if (failed.isError) {
        return failed;
      } else {
        const catalog = JSON.parse(result.stdout.trim()) as Catalog;
        const keywords = listed.keywords
          ? parseKeywordsTable(await readFile(listed.keywords, "utf8"))
          : [];
        return {
          content: [{ type: "text", text: JSON.stringify(inferPatterns(catalog, keywords), null, 2) }],
        };
      }
    },
  );

  for (const resource of RESOURCES) {
    const file = path.join(resourcesDir, resource.file);
    server.registerResource(
      resource.name,
      resource.uri,
      {
        title: resource.title,
        description: resource.title,
        mimeType: "text/markdown",
      },
      async (uri) => ({
        contents: [
          {
            uri: uri.href,
            mimeType: "text/markdown",
            text: await readFile(file, "utf8"),
          },
        ],
      }),
    );
  }

  server.registerPrompt(
    "add-model",
    {
      title: "Add a dtct model",
      description: "Add a model to the contract, document keywords, and wire ngin if needed.",
      argsSchema: z.object({
        name: z.string().describe("Model name, e.g. Event"),
        platforms: z.string().describe("Comma-separated platforms: api_server, web_frontend, mobile_frontend"),
      }),
    },
    ({ name, platforms }) => ({
      messages: [
        {
          role: "user" as const,
          content: {
            type: "text" as const,
            text: `Add dtct model ${name} for platforms: ${platforms}. Read datuma://language/dtct and datuma://language/standards. Prefer Data/Enum, standard types, and flat flags (relationship<model(X), BelongsTo, Select>). Type is not the widget. Call infer_patterns; treat association_suggestions as review notes, not validate failures. Add keywords.md rows, then advise_ngin. Do not invent ngin for a platform that does not exist yet.`,
          },
        },
      ],
    }),
  );

  server.registerPrompt(
    "add-field",
    {
      title: "Add a field and wire templates",
      description: "Add a field to an existing model and update ngin templates that should emit it.",
      argsSchema: z.object({
        model: z.string(),
        field: z.string(),
      }),
    },
    ({ model, field }) => ({
      messages: [
        {
          role: "user" as const,
          content: {
            type: "text" as const,
            text: `Add field ${field} to model ${model}. Update keywords.md. Find .ngin files that iterate that model's fields and ensure they will emit the new field. validate, then preview, then generate only after preview looks right.`,
          },
        },
      ],
    }),
  );

  server.registerPrompt(
    "scaffold-ngin",
    {
      title: "Scaffold an ngin target",
      description: "Write a new .ngin template for a platform that should stay in sync with the contract.",
      argsSchema: z.object({
        target: z.string().describe("Output folder or platform, e.g. python API or React web"),
      }),
    },
    ({ target }) => ({
      messages: [
        {
          role: "user" as const,
          content: {
            type: "text" as const,
            text: `Scaffold an ngin template for ${target}. Read datuma://language/ngin and datuma://docs/when-ngin. Reuse definition/*.def.ngin helpers. Emit only contract-derived types/validation/UI. Keep routing and auth handwritten.`,
          },
        },
      ],
    }),
  );

  server.registerPrompt(
    "should-use-ngin",
    {
      title: "Should I use ngin?",
      description: "Decide whether this work belongs in ngin or handwritten code.",
      argsSchema: z.object({
        task: z.string(),
      }),
    },
    ({ task }) => ({
      messages: [
        {
          role: "user" as const,
          content: {
            type: "text" as const,
            text: `Should this use ngin?\n\n${task}\n\nCall advise_ngin with the intended platforms. Read datuma://docs/when-ngin. Prefer splitting mixed tasks: ngin for contract-derived bits, handwritten glue between generated spans.`,
          },
        },
      ],
    }),
  );

  server.registerPrompt(
    "infer-contract-patterns",
    {
      title: "Infer contract patterns",
      description: "Run infer_patterns, apply shared traits/attributes, update keywords.md, then validate.",
      argsSchema: z.object({
        note: z.string().optional().describe("Optional extra context"),
      }),
    },
    ({ note }) => ({
      messages: [
        {
          role: "user" as const,
          content: {
            type: "text" as const,
            text: `Call infer_patterns. Read datuma://language/standards. Apply Data/Enum, cardinality/dependency flags, and explicit UI widgets on the contract (type is not the widget; do not stamp Text on every string). Treat association_suggestions as prevention/mitigation for review — the compiler does not reject them. For single-platform unique/relationship/email, keep the tag and list likely future platforms on the keywords.md row; do not add ngin until a second consumer exists. Then validate.${note ? `\n\n${note}` : ""}`,
          },
        },
      ],
    }),
  );

  return server;
}
