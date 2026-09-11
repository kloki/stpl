---
name: stpl
description: Create, find, read, search, list, and tag markdown memos/notes, and keep timestamped diaries, with the stpl CLI. Use when the user wants to jot a note, save a memo, capture standup/meeting notes, read or open an existing note, find a note by title, search note contents, append to a note, rename a note, add or remove tags, see an overview of their notes, keep a diary or journal, log what happened today, or add/read a dated diary entry.
allowed-tools: Bash Read
---

# Use `stpl`

`stpl` (staple) stores each memo as a plain `.md` file in a dated
`year/ISO-week/` tree under the configured memo directory (default `~/stpls`).
It also keeps **diaries** — append-only timestamped logs in `diaries/`, outside
the dated tree. Its output is agent-friendly. If `stpl --version` fails, run the
`/stpl:setup-stpl` skill first.

## Overview — list memos

List memos grouped by `year/week`.

```sh
stpl overview                    # plain text (default)
stpl overview json               # machine-readable — prefer this for parsing
stpl overview -a 2026-06-01 -b 2026-06-30   # filter by date range (inclusive)
stpl overview -t work            # only memos tagged `work`
```

- The output **format** is an optional positional argument: `text` (default),
  `json`, `markdown`, or `editor` — e.g. `stpl overview markdown`. **Avoid
  `editor`** in an agent context: it opens `$EDITOR` and blocks.
- `-a, --after` / `-b, --before` — `YYYY-MM-DD`, inclusive.
- `-t, --tag` — keep only memos with this tag (read from the frontmatter
  `tags: [..]` line). Repeatable, case-insensitive, ORs across tags. In `json`
  output each memo carries a `tags` array.

When you need to inspect what notes exist before acting, use
`stpl overview json` and parse it.

## New — create a memo

```sh
stpl new "Standup notes" -m "blocked on CI"
```

- **Always pass `-m/--message`** when creating a memo programmatically. Without
  `-m`, `stpl new` opens the file in `$EDITOR`, which blocks and cannot be
  driven non-interactively.
- The memo is dated today; the filename slug is derived from the title.
- The command reports the absolute path of the created file.

## Path — locate / read a memo

Print the absolute path of a memo, fuzzy-matched by title — nothing else, so it
composes well in scripts.

```sh
stpl path standup                # prints e.g. /home/you/stpls/2026/24/2026-06-14-standup-notes.md
stpl path -d standup             # print the containing folder instead of the file
```

Fuzzy matching: an exact (case-insensitive) title or slug always wins;
otherwise the closest match is used. If several memos match closely, `stpl`
lists the candidates and asks you to be more specific rather than guessing — in
that case, re-run with a more specific title.

## Omitting the title

`edit`, `path`, `show`, `append`, `del`, and `expand` accept the title
optionally — with no title they act on the **last memo used** (remembered in
`~/.local/state/stpl/last.toml`, updated by every command that targets a memo).

**Always pass an explicit title in agent context.** The implicit target depends
on whatever ran last, including commands the user ran in another terminal, so
omitting it makes a command non-deterministic. It exists for interactive use.

With no title and nothing remembered, the command fails with
`no memo given and no recent memo remembered`.

## Show — read a memo's contents

Print a matched memo to stdout with no decoration — **prefer this over
`cat "$(stpl path …)"`** for reading a note.

```sh
stpl show standup                 # full file (frontmatter + body)
stpl show standup --no-frontmatter   # body only, skipping the YAML frontmatter
```

## Search — full-text search across bodies

Find memos by their **contents** (case-insensitive substring), not just title.
Same date/tag filters as `overview`.

```sh
stpl search "blocked on CI"       # text output: clickable memo + matching lines
stpl search login -f json         # machine-readable — prefer this for parsing
stpl search todo -t work -a 2026-06-01   # combine with tag / date filters
```

In `json`, each hit carries the memo fields plus a `matches` array of
`{ line, text }`. Avoid `-f editor` in an agent context (opens `$EDITOR`).

Search also covers **diary** entries. A diary hit has `"kind": "diary"` with
`name`, `slug`, and `path` but no `date`, `week`, or `tags`, and each of its
matches adds an `entry` timestamp naming the entry the line sits in — so branch
on `kind` when parsing. `-a`/`-b` filter diary matches by their entry's date;
`-t` excludes diaries entirely, since they carry no tags.

## Append — add to a memo without an editor

```sh
stpl append standup -m "CI is green again"
```

Appends the message as a new line (after a blank separator), leaving the file
tidy. Non-interactive — safe to drive programmatically.

## Diary — timestamped running logs

A diary is one file holding many entries, each under a `# <ISO timestamp>`
heading. Use it for a journal, a work log, or anything append-only; use a memo
when the note is about one topic.

```sh
stpl diary work -m "standup: blocked on CI"   # append an entry (creates it if new)
stpl diary list json                          # every diary — prefer this for parsing
stpl diary show work                          # print the whole diary
```

- **Always pass `-m/--message` and an explicit name.** Without `-m` the command
  opens `$EDITOR` and blocks; without a name it targets whatever diary was used
  last, which is non-deterministic.
- Adding takes the name **exactly** (slugified, so `Work` and `work` are one
  diary) and creates it on first use. `show`, `edit`, `path`, and `del` instead
  fuzzy-match the name like memo titles.
- `stpl diary list json` returns `{ name, slug, path, entries, last_entry }` per
  diary; `last_entry` is `null` for an empty one.
- `list`, `show`, `edit`, `path`, and `del` are subcommand names. For a diary
  actually called one of them, use `stpl diary -m "text" -- list`.
- Diaries are invisible to `overview` and the memo commands. `stpl search` does
  cover them — see above.

## Tag — add tags to a memo

Add one or more tags to a memo's frontmatter, fuzzy-matched by title. Duplicates
(already-present tags) are ignored, so it is safe to re-run.

```sh
stpl tag standup work urgent     # add the `work` and `urgent` tags
```

- Pass the title first, then one or more space-separated tags (at least one is
  required).
- Tags written here are what `stpl overview -t <tag>` filters on.
- Title fuzzy-matching works the same as `path` above.
- `stpl untag <title> <tags>...` removes tags (case-insensitive; missing tags
  ignored). `stpl tags` lists every tag with its memo count (`stpl tags json`
  for parsing).

## Related commands

- `stpl edit <title>` — open a matched memo in `$EDITOR` (interactive; avoid in
  agent context — prefer `stpl show <title>` to read).
- `stpl rename <title> <new-title>` — re-slug and move a memo (file or project),
  keeping its date/folder, and rewrite the in-file title.
- `stpl del <title> [-y]` — delete a memo; `-y` skips confirmation (required
  without a TTY).
- `stpl expand <title>` — turn a single-file memo into a project directory.
- `rename`, `tag`, and `untag` always require an explicit title — they take a
  second argument, so it cannot be omitted.
- `stpl diary <name> -m "text"` — append a timestamped diary entry (see above).
- `stpl sync` — commit, pull, and push the memo directory, diaries included
  (git-backed).
