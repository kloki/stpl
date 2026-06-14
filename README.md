# 📎 stpl (staple)

Quick creation and management of markdown notes and memos from the command line.

`stpl` is **filesystem-based** — every memo is a plain `.md` file in a dated
folder tree, so your notes stay yours, readable and editable with any tool
(`nvim`, `grep`, `rg`, git…). It's designed to be equally pleasant for **humans**
and **agentic AI**: clean text output, clickable `file://` links, fuzzy title
matching, and structured JSON when you need it.

## Install

### Binaries

Check [Releases](https://github.com/kloki/stpl/releases) for binaries and installers.

## Quick start

```sh
stpl init                              # write ~/.config/stpl.toml
stpl new "Standup notes" -m "blocked on CI"
stpl overview                          # list everything, grouped by week
stpl edit standup                      # fuzzy-match and open in $EDITOR
```

## How notes are stored

Memos live under the configured memo directory (default `~/stpls`) in a
`year / ISO-week / file` tree. Folders are created lazily, on demand.

```
~/stpls/
└── 2026/
    └── 24/                                  # ISO week number, zero-padded
        ├── 2026-06-14-standup-notes.md      # a memo
        └── 2026-06-14-release/              # a project (see `expand`)
            └── project.md
```

- Filenames are `<iso-date>-<slug>.md`; the slug is a lower-kebab form of the
  title (`Standup Notes` → `standup-notes`).
- Weeks use the **ISO-8601** week (and ISO week-numbering year), so notes near a
  year boundary group consistently.

A new memo starts from a small template:

```markdown
---
title: Standup notes
date: 2026-06-14
tags: []
---

# Standup notes

blocked on CI
```

## Configuration

`stpl init` writes `~/.config/stpl.toml`:

```toml
memo_directory = "/home/you/stpls"
disable_emoji  = false
disable_color  = false
```

| Option           | Description                                                 |
| ---------------- | ----------------------------------------------------------- |
| `memo_directory` | Root directory for all memos. `~` is expanded.              |
| `disable_emoji`  | Drop emoji (e.g. 📎) from output.                           |
| `disable_color`  | Disable ANSI color. The `NO_COLOR` env var is also honored. |

Color and clickable links are automatically suppressed when output is not a
terminal (e.g. piped to a file or another program).

## Commands

### `stpl init`

Create the config file with defaults. Won't overwrite an existing config.

### `stpl new <title> [-m <content>]`

Create a new memo dated today.

- With `-m/--message`, writes the content and reports the path.
- Without `-m`, creates the file and opens it in `$EDITOR`.

```sh
stpl new "Grocery list" -m "milk, eggs, coffee"
stpl new "Design doc"                     # opens $EDITOR
```

### `stpl edit <title>`

Fuzzy-match a memo by title and open it in `$EDITOR`.

### `stpl path <title>`

Fuzzy-match a memo by title and print its absolute path — nothing else. Handy
for scripting and agentic AI:

```sh
cat "$(stpl path standup)"
nvim "$(stpl path 'design doc')"
```

### `stpl del <title> [-y]`

Delete a memo after confirmation. Projects remove the whole directory (the
prompt makes this explicit). Pass `-y/--yes` to skip the prompt; without a TTY,
`-y` is required.

### `stpl expand <title>`

Turn a single-file memo into a **project** directory:
`…-release.md` → `…-release/project.md`. Useful when a note grows into something
with attachments or multiple files.

### `stpl sync`

Keep your notes in sync through a git remote (e.g. GitHub), assuming the memo
directory is a git repository. In one command it runs **commit → pull → push**:

1. `git add -A` and commit any local changes with a timestamped message
   (skipped when there's nothing to commit).
2. `git pull` to integrate remote changes. If the pull hits merge conflicts,
   `stpl` stops and tells you to resolve them in the memo directory, then run
   `stpl sync` again — it does **not** push a conflicted tree.
3. `git push` to publish.

If the memo directory isn't a git repo yet, `stpl sync` prints the one-time
setup steps (`git init`, add a remote, initial commit, `push -u`) instead.

```sh
stpl sync
```

### `stpl overview [-f <format>] [-a <after>] [-b <before>]`

List memos grouped by `year/week`.

- `-f, --format` — `text` (default, agent-friendly), `json`, `markdown`, or
  `editor` (renders markdown and opens it in `$EDITOR`).
- `-a, --after` / `-b, --before` — filter by date, inclusive, `YYYY-MM-DD`.

```sh
stpl overview                            # text
stpl overview -f json                    # machine-readable
stpl overview -a 2026-06-01 -b 2026-06-30
```

## Title matching

Title arguments to `edit`, `path`, `del`, and `expand` are **fuzzy-matched** against
existing memos:

- An exact (case-insensitive) title or slug always wins.
- Otherwise the closest match is used.
- If several memos match closely, `stpl` lists the candidates (as clickable
  links) and asks you to be more specific rather than guessing.
