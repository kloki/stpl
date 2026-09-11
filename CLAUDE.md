# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`stpl` (staple) is a single-binary Rust CLI for managing markdown notes/memos. It is
**filesystem-based**: every memo is a plain `.md` file in a dated folder tree, so notes
stay readable/editable with any tool. Alongside memos it manages **diaries** — long-lived
append-only logs under `diaries/`, outside the dated tree. Output is designed to be equally
pleasant for humans and agentic AI (clean text, clickable `file://` links via OSC 8, fuzzy
title matching, and structured JSON via `overview -f json`).

See `README.md` for the full user-facing command reference.

## Commands

```sh
cargo build              # debug build
cargo build --release    # release build
cargo run -- <args>      # e.g. cargo run -- overview -f json
cargo test               # run all tests
cargo test <name>        # run tests matching a substring, e.g. `cargo test slugify`
cargo fmt
cargo clippy
```

Tests are colocated in `#[cfg(test)] mod tests` blocks within each source file (notably
`memo.rs`, `store.rs`, `diary.rs`, `config.rs`). Filesystem tests write to unique `temp_dir()` paths.

Package installs resolve through the internal Nexus (already configured) — do not repoint
to public registries.

## Architecture

The crate is a thin clap dispatcher (`main.rs` → `cli.rs` → `commands::*::run`) over a few
focused modules. Commands hold orchestration only; all real logic lives in the shared
modules below, which is where most changes belong.

- **`memo.rs`** — the `Memo` data model and all path↔memo conversion. This is the heart of
  the storage convention. A memo is either a `File` (`<iso-date>-<slug>.md`) or a `Project`
  (`<iso-date>-<slug>/project.md`). `Memo::from_path` parses either shape and returns `None`
  for anything that doesn't fit (callers skip silently). Pure helpers: `slugify`,
  `title_from_slug`, `stem_for`, `dir_for`, `memo_path`, plus frontmatter `tags` parsing.
- **`store.rs`** — all *memo* filesystem mutation/enumeration: `list_all` (walks the tree,
  skips unparseable files and prunes `diaries/`, returns `[]` when the root is absent),
  `create`, `delete`, `expand`, `render_template`.
- **`diary.rs`** — the whole diary feature below the command layer: the `Diary` model,
  `diary_path`, the pure entry helpers (`format_entry`, `append_entry`, `parse_heading`,
  `parse_entries`), the filesystem ops (`list_all`, `add_entry`, `read_content`, `delete`),
  and `resolve_one`. Diary fs ops live here rather than in `store.rs` so the two storage
  conventions stay separable.
- **`resolve.rs`** — `pick`, the generic matcher (exact case-insensitive title/slug match
  wins; otherwise `SkimMatcherV2` scores, and the result is `NotFound`, a single match, or
  `Ambiguous` when the top score doesn't beat #2 by ≥1.5×), plus `resolve_one` which applies
  it to memos. `diary::resolve_one` applies the same `pick` to diary names.
  `edit`/`path`/`del`/`expand` all route titles through it.
- **`config.rs`** — `Config` (TOML at `~/.config/stpl.toml`), defaults, `~` expansion, and
  `stpl init`. `memo_directory` is always absolute by the time a `Config` exists.
- **`state.rs`** — the "last memo used" pointer (TOML at `~/.local/state/stpl/last.toml`)
  and the independent "last diary used" pointer (`last-diary.toml`), which is what lets
  `edit`/`path`/`show`/`append`/`del`/`expand` and the diary commands take their target
  optionally. Stores only the path; `load` rehydrates via `Memo::from_path` (or
  `Diary::from_path`) and returns `None` when it is gone, so stale pointers are
  self-detecting.
- **`output.rs`** — presentation: `Style::from_config` resolves color/hyperlink
  capability from config + `NO_COLOR` + TTY detection. `memo_line` renders the canonical
  `- title[file://…]` clickable line; `success`/`print_error` handle messaging.
- **`editor.rs`** — launches `$EDITOR`/`$VISUAL`/`vi`, refusing when there is no TTY.
- **`error.rs`** — `StplError`, the domain error enum. Commands return `anyhow::Result`;
  `main` prints the full cause chain in red and exits non-zero.

### Key invariants

- **ISO weeks everywhere.** The folder tree is `<year>/<week>` using
  `date.iso_week().year()` (NOT calendar `date.year()`) and a zero-padded 2-digit week, so
  notes near a year boundary group consistently. `dir_for` is the single source of truth —
  use it rather than rebuilding paths.
- **Lazy directories.** `<year>/<week>` folders are created on demand by `create`; nothing
  pre-creates the memo root, and `list_all` treats a missing root as empty.
- **Graceful skipping.** Foreign/non-conforming files (wrong name, multibyte stem straddling
  the date boundary, etc.) must be silently ignored, never panic — see the `from_path` /
  `parse_stem` tests.
- **`from_path`/`memo_path` round-trip.** Changing the on-disk naming scheme means updating
  both, plus the round-trip tests in `memo.rs`.
- **`diaries/` is reserved.** It sits directly under the memo root and holds one `.md` per
  diary. Memo enumeration must never descend into it — `store::list_all` prunes it at
  depth 1 — or a diary named like a date-prefixed memo would show up in `overview`.
- **Diary headings are `# %Y-%m-%dT%H:%M` in local time.** `diary::parse_heading` is the
  single source of truth for what opens an entry; both `parse_entries` and `stpl search`
  go through it. Changing the format means changing that one function.
- **Adding a diary entry takes the name exactly; managing one fuzzy-matches.** Creation is
  on demand, so a fuzzy match on add would route a typo into an existing diary.
- **The two state pointers are independent files.** `del` and `diary del` each clear only
  their own, and neither command can clobber the other's.
- **State is best-effort.** Nothing in `state.rs` may fail a command — an unwritable state
  dir is a silent no-op. Commands resolve their target through `commands::util`:
  `resolve_or_last` (resolve + record) for the normal case, `resolve_target` (resolve only)
  for `del`, which must not record a memo it is about to delete.

### `CONTRACT` headers

Several modules (`memo.rs`, `store.rs`, `diary.rs`, `resolve.rs`, `config.rs`, `output.rs`,
`editor.rs`)
carry a `CONTRACT — implement the bodies; do not change public signatures` note. Treat
those public function signatures as stable APIs; change behavior within them rather than
reshaping the interface.

## Distribution & plugin

- Releases are built with `cargo-dist` (`dist-workspace.toml`, `.github/workflows/release.yml`)
  for linux gnu/musl targets; installers are shell-based.
- `claude-plugin/` ships a Claude Code plugin (skills `setup-stpl` and `stpl`), surfaced via
  `.claude-plugin/marketplace.json`.
