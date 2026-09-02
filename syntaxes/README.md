# datuma_k TextMate grammars

VS Code / Cursor language extension for `.dtct` and `.ngin`. Workspace settings cannot register TextMate grammars; install this folder once.

## Install

1. Command Palette → **Extensions: Install from Location…**
2. Choose this `syntaxes` directory (the folder that contains `package.json`).

## Reload after edits

The editor does **not** hot-reload an installed TextMate grammar when you save the JSON.

1. Edit files under this `syntaxes` folder (the same folder you installed from).
2. Command Palette → **Developer: Reload Window**.

If a token has the wrong color: **Developer: Inspect Editor Tokens and Scopes** (try `ProductCatalog`, `sku`, `product_code` in a `.dtct` file).
