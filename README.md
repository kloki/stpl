# 📎 stpl (staple)

Mgmt notes and memos from the command line that work for me (and agentic ai).

`stpl` is **filesystem-based** — every memo is a plain `.md` file in a dated
folder tree, so your notes stay yours, readable and editable with any tool
(`nvim`, `grep`, `rg`, git…). It's designed to be equally pleasant for **humans**
and **agentic AI**: clean text output, clickable `file://` links, fuzzy title
matching, and structured JSON when you need it.

## Install

### Binaries

Check [Releases](https://github.com/kloki/stpl/releases) for binaries and installers.

## Claude Code plugin

This repo ships a [Claude Code](https://claude.com/claude-code) plugin so Claude
can drive `stpl` for you. It bundles two skills:

- **`/stpl:setup-stpl`** — install `stpl` and create/configure its config.
- **`/stpl:stpl`** — day-to-day use: `overview` (list), `new` (create), and
  `path` (find/read) memos.

Install it from this repo's marketplace:

```sh
claude plugin marketplace add https://github.com/kloki/stpl
claude plugin install stpl@stpl
# then, inside Claude Code:
/stpl:setup-stpl
/stpl:stpl
```

## Quick start

```sh
stpl init                              # write ~/.config/stpl.toml
stpl new "Standup notes" -m "blocked on CI"
stpl overview                          # list everything, grouped by week
stpl edit standup                      # fuzzy-match and open in $EDITOR
```

For more

```
stpl --help
stpl overview --help
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
```

## Title matching

Title arguments to `edit`, `path`, `del`, and `expand` are **fuzzy-matched** against
existing memos:

- An exact (case-insensitive) title or slug always wins.
- Otherwise the closest match is used.
- If several memos match closely, `stpl` lists the candidates (as clickable
  links) and asks you to

## Configuration

`stpl init` writes `~/.config/stpl.toml`:

```toml
memo_directory = "/home/you/stpls"
disable_emoji  = false
disable_color  = false
```

be more specific rather than guessing.
