---
name: setup-stpl
description: Install and configure the stpl (staple) markdown-memo CLI. Use when the user wants to install stpl, set up stpl, get stpl working, create its config, or choose where memos are stored.
allowed-tools: Bash Read Edit
---

# Set up `stpl`

Get the `stpl` CLI installed and configured on this machine. Work through the
steps in order; skip any that are already satisfied.

## 1. Check whether `stpl` is already installed

```sh
stpl --version
```

If this prints a version, `stpl` is on `PATH` — skip to step 3 (config).

## 2. Install the binary

Offer the user the option that fits their situation:

- **Release binary (no toolchain needed):** download from
  <https://github.com/kloki/stpl/releases> and place it on `PATH`.
- **From this repo (when working inside the stpl checkout):**
  ```sh
  cargo install --path .
  ```
- **From git (anywhere):**
  ```sh
  cargo install --git https://github.com/kloki/stpl
  ```

> Note: package installs resolve through the configured internal registry
> (Nexus). Do **not** repoint cargo/npm/pip at public registries; if an install
> fails, report it instead of changing the registry.

Re-run `stpl --version` to confirm it is now on `PATH`.

## 3. Create the config

```sh
stpl init
```

This writes `~/.config/stpl.toml` with defaults. It will **not** overwrite an
existing config, so it is safe to run.

## 4. Configure (optional)

`~/.config/stpl.toml`:

```toml
memo_directory = "/home/you/stpls"   # root for all memos; ~ is expanded
disable_color  = false               # disable ANSI color (NO_COLOR also honored)
```

Ask the user where they want memos stored and update `memo_directory` if they
want something other than the default `~/stpls`. Color and clickable links are
auto-suppressed when output is not a terminal.

## 5. Optional: git-backed sync

If the user wants memos synced across machines via a git remote, make the memo
directory a git repo and use `stpl sync` (commit → pull → push). Running
`stpl sync` in a non-git memo directory prints the one-time setup steps.

## 6. Verify

```sh
stpl overview
```

A clean (possibly empty) listing means setup succeeded. The `/stpl:stpl` skill
covers day-to-day usage.
