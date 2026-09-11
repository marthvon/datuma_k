# datuma_k MCP

stdio MCP server for agents **using** the installed `datuma_k` binary in an app repo. It does not link the compiler crate. Set `DATUMA_K` to the binary if it is not on `PATH`.

## Install

Published to npm as [`@marthvon/datuma-k-mcp`](https://www.npmjs.com/package/@marthvon/datuma-k-mcp) and to the [MCP Registry](https://registry.modelcontextprotocol.io) as `io.github.marthvon/datuma-k`.

Cursor / Claude example (`mcp.json`):

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

`datuma_k` must be on `PATH`. If it is not, point `DATUMA_K` at the binary:

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

### From source

```bash
cd mcp
npm install
npm run build
```

Then use `"command": "node"` with `"args": ["/ABS/PATH/TO/datuma_k/mcp/dist/index.js"]`.

The process cwd should be the datuma_k **project** (the directory with `.env`), or pass `root` on each tool call.

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

## Resources

- `datuma://language/dtct`
- `datuma://language/ngin`
- `datuma://language/standards`
- `datuma://docs/keywords`
- `datuma://docs/practices`
- `datuma://docs/when-ngin`

## Prompts

`add-model`, `add-field`, `scaffold-ngin`, `should-use-ngin`, `infer-contract-patterns`

## Tests

```bash
npm test
```

## Publishing

Two registries, in this order. npm hosts the tarball; the [MCP Registry](https://registry.modelcontextprotocol.io) hosts only metadata and verifies it against the published npm package, so npm must go first or the second publish fails with `Registry validation failed for package`.

| Name | Where | Set in |
| --- | --- | --- |
| `@marthvon/datuma-k-mcp` | npm package | `package.json` `name` |
| `io.github.marthvon/datuma-k` | MCP Registry server | `package.json` `mcpName` **and** `server.json` `name` — these two must match, or the registry rejects the publish |

The `io.github.marthvon/` prefix is not cosmetic: GitHub device-flow auth only grants that namespace. A different prefix needs [DNS or HTTP auth](https://modelcontextprotocol.io/registry/authentication).

### One-time setup

```bash
brew install mcp-publisher            # or a binary from the registry releases page
npm login                             # npm account, scope @marthvon
```

npm requires 2FA to publish. If the account does not have it, `npm publish` fails with `403 ... Two-factor authentication or granular access token with bypass 2fa enabled is required`, before it ever looks at the tarball. Check with `npm profile get`; enable with `npm profile enable-2fa auth-and-writes` or on npmjs.com, and keep the recovery codes.

### Release

Bump the version in **three** places — the registry rejects a version it has already seen, and a mismatch between the npm package version and `server.json` `packages[0].version` fails verification:

- `package.json` → `version`
- `server.json` → `version`
- `server.json` → `packages[0].version`

Then:

```bash
npm test                              # tsc + node --test
mcp-publisher validate                # checks server.json against the live registry, publishes nothing
npm publish --otp=123456              # prepublishOnly runs the build; drop --otp to be prompted
mcp-publisher login github            # device code → github.com/login/device
mcp-publisher publish
```

`mcp-publisher login` persists a token, so day-to-day releases are `npm publish` then `mcp-publisher publish`.

Verify:

```bash
npm view @marthvon/datuma-k-mcp version
curl "https://registry.modelcontextprotocol.io/v0.1/servers?search=io.github.marthvon/datuma-k"
```

### What ships

`files` in `package.json` limits the tarball to `dist`, `resources`, and `README.md`. `resources/` is **required at runtime** — `server.ts` resolves it as `dist/../resources` — so dropping it from `files` produces a package that installs cleanly and fails on every resource read. Confirm with `npm pack --dry-run` before publishing.

### Troubleshooting

| Error | Cause |
| --- | --- |
| `403 ... Two-factor authentication ... is required` | 2FA disabled on the npm account. See above. |
| `403 ... you do not have access` | The `@marthvon` scope belongs to an org; confirm membership with `npm org ls marthvon`. |
| `Registry validation failed for package` | npm publish did not happen yet, or `mcpName` in `package.json` does not match `name` in `server.json`. |
| `Invalid or expired Registry JWT token` | Re-run `mcp-publisher login github`. |
| `You do not have permission to publish this server` | Server name does not start with `io.github.<your-github-username>/`. |
| `expected length <= 100` on `body.description` | `server.json` `description` is capped at 100 characters. The npm `description` is not. |
