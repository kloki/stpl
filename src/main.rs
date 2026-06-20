//! stpl (staple) — quick creation and management of markdown notes/memos.

mod cli;
mod commands;
mod config;
mod editor;
mod error;
mod memo;
mod output;
mod resolve;
mod store;

use std::process;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        // Failed commands print in red on stderr.
        output::print_error(&err);
        process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Init => commands::init::run(),
        Command::New { title, message } => commands::new::run(&title, message.as_deref()),
        Command::Edit { title } => commands::edit::run(&title),
        Command::Path { title, dir } => commands::path::run(&title, dir),
        Command::Show {
            title,
            no_frontmatter,
        } => commands::show::run(&title, no_frontmatter),
        Command::Append { title, message } => commands::append::run(&title, &message),
        Command::Rename { title, new_title } => commands::rename::run(&title, &new_title),
        Command::Search {
            query,
            format,
            after,
            before,
            tags,
        } => commands::search::run(&query, format, after.as_deref(), before.as_deref(), &tags),
        Command::Sync => commands::sync::run(),
        Command::Del { title, yes } => commands::del::run(&title, yes),
        Command::Expand { title } => commands::expand::run(&title),
        Command::Tag { title, tags } => commands::tag::run(&title, &tags),
        Command::Untag { title, tags } => commands::untag::run(&title, &tags),
        Command::Tags { format } => commands::tags::run(format),
        Command::Overview {
            format,
            after,
            before,
            tags,
        } => commands::overview::run(format, after.as_deref(), before.as_deref(), &tags),
    }
}
