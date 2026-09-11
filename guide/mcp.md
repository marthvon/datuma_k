# MCP server

`@marthvon/datuma-k-mcp` is a stdio [MCP](https://modelcontextprotocol.io) server that gives a coding agent the datuma_k CLI as structured tools. The agent can read the contract, validate it, preview a run, and generate — instead of shelling out and parsing text.

It **uses** the installed `datuma_k` binary. It does not link the compiler crate, so the binary must be installed first: [Getting started](getting-started.md#install).

## Install

No install step. Every client below runs it with `npx`, which fetches the package on first use.

### Claude Code

```bash
claude mcp add datuma_k -- npx -y @marthvon/datuma-k-mcp
```

Add `--scope project` to write `.mcp.json` in the repo so the team shares it.

### Claude Desktop / Cursor / VS Code

`claude_desktop_config.json`, `.cursor/mcp.json`, or `.vscode/mcp.json`:

```json
{
  "mcpServers": {
    "datuma_k": {
      "command": "npx",
      "args": ["-y", "@marthvon/datuma-k-mcp"]
    }
  }
}
```

### From the MCP Registry

The server is listed as `io.github.marthvon/datuma-k`. Clients that read the registry can install it by that name.

## The binary

The server calls `datuma_k` on `PATH`. If it is not there — a `cargo build` checkout, or a release binary you did not install system-wide — point `DATUMA_K` at it:

```json
{
  "mcpServers": {
    "datuma_k": {
      "command": "npx",
      "args": ["-y", "@marthvon/datuma-k-mcp"],
      "env": {
        "DATUMA_K": "/ABS/PATH/TO/datuma_k/target/release/datuma_k"
      }
    }
  }
}
```

## The project root

Tools run against a datuma_k project — the directory holding `.env`. The server uses its own working directory, which is whatever the client launched it in. That is usually the folder you opened in the editor, so it is right by default and wrong the moment the project lives in a subdirectory.

Either pass `root` on each tool call, or launch the server in the right place. Most clients accept a `cwd`:

```json
{
  "mcpServers": {
    "datuma_k": {
      "command": "npx",
      "args": ["-y", "@marthvon/datuma-k-mcp"],
      "cwd": "/ABS/PATH/TO/project"
    }
  }
}
```

## Tools

| Tool | Binary | Notes |
| --- | --- | --- |
| `list_project` | (files + `.env`) | Directories and contract/template paths |
| `query_contracts` | `datuma_k catalog` | Optional `trait` / `model` / `field` / `attribute` / `type` |
| `validate` | `datuma_k check` | Parse + `data/keywords.md` required |
| `preview` | `datuma_k preview` | Plan without writing |
| `generate` | `datuma_k run` | Writes generated spans |
| `advise_ngin` | (local rules) | When to use ngin vs handwritten code |
| `infer_patterns` | `catalog` + `keywords.md` | Traits, widgets, association review notes, sync risks, aliases |

`generate` is the only tool that writes to disk. `preview` produces the same plan without touching files — worth making the agent run it first on an unfamiliar project.

## Resources

The language and practice docs, served to the agent on request:

- `datuma://language/dtct`
- `datuma://language/ngin`
- `datuma://language/standards`
- `datuma://docs/keywords`
- `datuma://docs/practices`
- `datuma://docs/when-ngin`

These ship inside the package, so they match the server version rather than whatever is in your checkout.

## Prompts

`add-model`, `add-field`, `scaffold-ngin`, `should-use-ngin`, `infer-contract-patterns`

Each one loads the relevant resources and sets the house rules — that type is not the widget, that `association_suggestions` are review notes rather than errors, that routing and auth stay handwritten. Invoking the prompt beats describing the task cold.

## Troubleshooting

| Symptom | Cause |
| --- | --- |
| Every tool reports no contracts | Wrong working directory. Pass `root`, or set `cwd` in the client config. |
| `datuma_k: command not found` | Binary not on `PATH`. Set `DATUMA_K` to its absolute path. |
| `validate` fails on a contract that compiles | `data/keywords.md` is missing or stale — `check` requires it. |
| Server never starts | Node older than 20, or `npx` has no network on first run. |

## Developing on it

The source is in [`mcp/`](../mcp). Build and test locally:

```bash
cd mcp
npm install
npm run build
npm test
```

Point a client at the local build with `"command": "node"` and `"args": ["/ABS/PATH/TO/datuma_k/mcp/dist/index.js"]`. Release steps are in [`mcp/README.md`](../mcp/README.md#publishing).
