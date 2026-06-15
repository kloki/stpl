---
name: stpl
description: Create, find, list, and tag markdown memos/notes with the stpl CLI. Use when the user wants to jot a note, save a memo, capture standup/meeting notes, read or open an existing note, find a note by title, add tags to a note, or see an overview of their notes.
allowed-tools: Bash Read
---

# Use `stpl`

`stpl` (staple) stores each memo as a plain `.md` file in a dated
`year/ISO-week/` tree under the configured memo directory (default `~/stpls`).
Its output is agent-friendly. If `stpl --version` fails, run the
`/stpl:setup-stpl` skill first.

## Overview — list memos

List memos grouped by `year/week`.

```sh
stpl overview                    # plain text (default)
stpl overview -f json            # machine-readable — prefer this for parsing
stpl overview -a 2026-06-01 -b 2026-06-30   # filter by date range (inclusive)
stpl overview -t work            # only memos tagged `work`
```

- `-f, --format` — `text` (default), `json`, `markdown`, or `editor`. **Avoid
  `editor`** in an agent context: it opens `$EDITOR` and blocks.
- `-a, --after` / `-b, --before` — `YYYY-MM-DD`, inclusive.
- `-t, --tag` — keep only memos with this tag (read from the frontmatter
  `tags: [..]` line). Repeatable, case-insensitive, ORs across tags. In `json`
  output each memo carries a `tags` array.

When you need to inspect what notes exist before acting, use
`stpl overview -f json` and parse it.

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
cat "$(stpl path standup)"       # read a memo's contents
```

Fuzzy matching: an exact (case-insensitive) title or slug always wins;
otherwise the closest match is used. If several memos match closely, `stpl`
lists the candidates and asks you to be more specific rather than guessing — in
that case, re-run with a more specific title.

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

## Related commands

- `stpl edit <title>` — open a matched memo in `$EDITOR` (interactive; avoid in
  agent context — prefer `cat "$(stpl path <title>)"` to read).
- `stpl del <title> [-y]` — delete a memo; `-y` skips confirmation (required
  without a TTY).
- `stpl expand <title>` — turn a single-file memo into a project directory.
- `stpl sync` — commit, pull, and push the memo directory (git-backed).
