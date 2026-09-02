# datuma_k MCP

stdio MCP server for agents **using** the installed `datuma_k` binary in an app repo. It does not link the compiler crate. Set `DATUMA_K` to the binary if it is not on `PATH`.

## Install

```bash
cd mcp
npm install
npm run build
```

Cursor / Claude example (`mcp.json`):

```json
{
  "mcpServers": {
    "datuma_k": {
      "command": "node",
      "args": ["/ABS/PATH/TO/datuma_k/mcp/dist/index.js"],
      "env": {
        "DATUMA_K": "/ABS/PATH/TO/datuma_k/target/release/datuma_k"
      }
    }
  }
}
```

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

## Resources

- `datuma://language/dtct`
- `datuma://language/ngin`
- `datuma://docs/keywords`
- `datuma://docs/practices`
- `datuma://docs/when-ngin`

## Prompts

`add-model`, `add-field`, `scaffold-ngin`, `should-use-ngin`

## Tests

```bash
npm test
```
