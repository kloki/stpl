# stpl (staple)

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
  `path` (find/read) memos, plus `diary` for timestamped logs.

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
stpl show standup                      # print contents to stdout (pipe-friendly)
stpl append standup -m "CI fixed"      # add a line without opening an editor
stpl search "blocked on CI"            # full-text search across memo bodies
stpl rename standup "Daily standup"    # re-slug and move, keeping the date
stpl tag standup work urgent           # add tags (duplicates ignored)
stpl untag standup urgent              # remove tags
stpl tags                              # list all tags with counts
stpl show                              # no title? act on the last memo you used
stpl diary work -m "CI fixed"          # append a timestamped entry to a diary
stpl diary -m "and again"              # same diary, no name needed
stpl diary list                        # every diary, with entry counts
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
├── 2026/
│   └── 24/                                  # ISO week number, zero-padded
│       ├── 2026-06-14-standup-notes.md      # a memo
│       └── 2026-06-14-release/              # a project (see `expand`)
│           └── project.md
└── diaries/                                 # diaries live outside the dated tree
    └── work.md
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

## Diaries

A diary is an append-only log you add to as the day goes: one file, many
timestamped entries. Where a memo is about a thing, a diary is about a stream.

```sh
stpl diary work -m "standup: blocked on CI"   # creates diaries/work.md
stpl diary work -m "CI fixed, merged the PR"  # a second entry, same file
stpl diary -m "heading home"                  # the last diary you used
```

You can keep as many as you like — `work`, `travel`, `health` — each its own
file under `diaries/`. The file is plain markdown with no frontmatter:

```markdown
# 2026-09-11T09:15

standup: blocked on CI

# 2026-09-11T14:32

CI fixed, merged the PR
```

Headings are local time to the minute, so several entries a day stay distinct.

Managing them mirrors the memo commands, and the name is optional everywhere
(it defaults to the last diary you used, tracked separately from the last memo):

```sh
stpl diary list          # every diary with its entry count and latest entry
stpl diary show work     # print the whole diary (pipe-friendly)
stpl diary edit work     # open it in $EDITOR
stpl diary path work     # print its absolute path
stpl diary del work -y   # delete it and all its entries
```

Two details worth knowing:

- **Adding takes the name exactly** (slugified, so `Work` and `work` are one
  diary), while `show`/`edit`/`path`/`del` fuzzy-match like memo titles. Adding
  has to create on demand, and a fuzzy match there would let a typo land in the
  wrong diary.
- **`list`, `show`, `edit`, `path`, `del`, and `help` are subcommand names**, so
  a diary called one of them needs `stpl diary -m "text" -- list`, or the
  explicit `stpl diary show list`.

Diaries stay out of the memo commands: `overview`, `show`, `edit`, `tag` and the
rest never see them. `stpl search` does search diary entries, showing each hit
with the timestamp of the entry it sits in; `-a`/`-b` filter by that entry's
date, and `-t` excludes diaries since they carry no tags. `stpl sync` commits
them along with everything else.

## Title matching

Title arguments to `edit`, `path`, `show`, `append`, `rename`, `del`, `expand`,
`tag`, and `untag` are **fuzzy-matched** against existing memos:

- An exact (case-insensitive) title or slug always wins.
- Otherwise the closest match is used.
- If several memos match closely, `stpl` lists the candidates (as clickable
  links) and asks you to

## The last memo you used

`edit`, `path`, `show`, `append`, `del`, and `expand` take the title
**optionally** — leave it out and they act on the last memo you used:

```sh
stpl new "Standup notes" -m "blocked on CI"
stpl append -m "CI fixed"   # same memo, no title needed
stpl show                   # still the same memo
stpl edit                   # open it in $EDITOR
```

Every command that targets a single memo updates that pointer — `new`, `edit`,
`path`, `show`, `append`, `rename`, `expand`, `tag`, and `untag`. `rename` and
`expand` follow the memo to its new location; `del` forgets it (and only it, so
deleting some other memo leaves the pointer alone).

`new` always needs a title, of course. So do `rename`, `tag`, and `untag`,
because they take a second argument — `stpl tag rust` can't tell a title from a
tag.

The pointer lives in `~/.local/state/stpl/last.toml` (XDG state dir), holding
just the memo's path. It's a convenience cache, so it's created lazily, never
errors, and if the memo has since been deleted or moved outside `stpl` you get:

```
error: no memo given and no recent memo remembered — pass a title, or create one with `stpl new <title>`
```

Diaries keep their own pointer in `~/.local/state/stpl/last-diary.toml`, so
`stpl diary -m "..."` and `stpl show` never interfere with each other.

Scripts and agents should keep passing explicit titles — an implicit target
depends on whatever ran last.

## Configuration

`stpl init` writes `~/.config/stpl.toml`:

```toml
memo_directory = "/home/you/stpls"
disable_color  = false
```

## Fuzzy finder

Add this snippet to you `.bashrc` to use `stpl` with `fzf`

```bash
sn() {
    local sel path cyan reset
    cyan=$(tput setaf 6)
    reset=$(tput sgr0)
    sel=$(stpl overview json | jq -r '.[].memos[] | [.title, (.tags | join(",")), .path] | @tsv' \
        | awk -F'\t' -v c="$cyan" -v r="$reset" '{print $1"\t"c$2r"\t"$3}' \
        | fzf --ansi --delimiter '\t' --with-nth=1,2 \
              --preview 'cat {3}' --preview-window=right:60%:wrap) || return
    path=$(echo "$sel" | awk -F'\t' '{print $3}')
    "$EDITOR" "$path"
}
```
