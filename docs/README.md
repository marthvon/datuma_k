# Documentation

| Section | Documents |
|---------|-----------|
| **Guides** | [Testing](guides/testing.md) · [Project overview](guides/project-overview.md) |
| **Core parser** | [Stack design](core/parser-stack-design.md) · [Module reference](core/parser-modules.md) |
| **DTCT** | [Implementation plan](dtct/implementation-plan.md) · [Module reference](dtct/modules.md) |
**Commands only:** [../README.md](../README.md) (build, test, run).

**Planning (WIP):** [../plan/](../plan/) — NGIN and future design notes.

## Source layout

```
src/core/parser/     Shared ParseMode / cursor / stack framework
src/dtct/            Data contract (*.dtct) parser + registry
src/ngin/            Template engine (*.py.ngin, …) — see plan/ngin/
```
