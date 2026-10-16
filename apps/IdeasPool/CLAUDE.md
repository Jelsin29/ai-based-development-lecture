# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

A learning repo for writing Claude Code plugins. It holds a Turkish step-by-step tutorial (`docs/plugin-rehberi.md`) and the example plugin it builds (`plugins/ideas/`). **The tutorial embeds every plugin file verbatim, so when you change a file under `plugins/ideas/`, update the matching code block in the tutorial too.**

## Commands

```bash
claude plugin validate ./plugins/ideas --strict      # check the manifest and skill/agent frontmatter
claude --plugin-dir ./plugins/ideas                  # start a session with the plugin loaded (no install)
claude --plugin-dir ./plugins/ideas plugin list      # confirm it loads (Status: ✔ loaded)
node plugins/ideas/scripts/session-summary.js        # run the SessionStart hook script by itself
```

Inside a session, run `/reload-plugins` after you edit plugin files.

## Plugin structure (`plugins/ideas/`)

- `.claude-plugin/plugin.json` is the manifest. Only the manifest goes in `.claude-plugin/`; component directories sit at the plugin root.
- `skills/add` and `skills/list` run as `/ideas:add` and `/ideas:list`. The plugin name is the prefix. `add` sets `disable-model-invocation: true` because it writes files.
- `agents/idea-reviewer.md` is a subagent limited to the `Read` and `Edit` tools. It sets an idea's `status` to `reviewed`.
- `hooks/hooks.json` runs `scripts/session-summary.js` on `SessionStart`, and the script's stdout goes into Claude's context. Hook scripts are written in Node, not bash, because `jq` isn't installed on this Windows machine. The script must always exit 0.
- Data lives in `ideas.json` in the user's project root (`CLAUDE_PROJECT_DIR`), not in the plugin directory. It's a JSON array of `{id, title, description, createdAt, status}`, where `status` is `new`, `reviewed` or `done`.
