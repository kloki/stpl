//! `stpl tags [format]` — list every tag with its memo count.

use std::{env, fs};

use anstyle::{AnsiColor, Color, Style as AnsiStyle};
use anyhow::Result;
use serde::Serialize;

use crate::{cli::Format, commands::util, editor, output, output::Style, store};

/// A single tag and how many memos carry it, serialized for `--format json`.
#[derive(Serialize)]
struct TagCount {
    tag: String,
    count: usize,
}

/// List all tags across every memo with their counts, sorted by count
/// descending then tag ascending.
pub fn run(format: Format) -> Result<()> {
    let (config, style) = util::config_and_style()?;
    let counts: Vec<TagCount> = store::tag_counts(&config)?
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();

    match format {
        Format::Text => render_text(&style, &counts),
        Format::Json => anstream::println!("{}", serde_json::to_string_pretty(&counts)?),
        Format::Markdown => anstream::println!("{}", render_markdown(&counts)),
        Format::Editor => {
            let path = env::temp_dir().join("stpl-tags.md");
            fs::write(&path, render_markdown(&counts))?;
            editor::open(&path)?;
        }
    }
    Ok(())
}

fn render_text(style: &Style, counts: &[TagCount]) {
    if counts.is_empty() {
        output::success(style, "no tags found");
        return;
    }
    for tc in counts {
        anstream::println!(
            "  {} {}",
            tag_colored(style, &tc.tag),
            dim(style, &format!("({})", tc.count))
        );
    }
}

/// `#tag` in magenta when color is enabled, else plain.
fn tag_colored(style: &Style, tag: &str) -> String {
    if !style.color {
        return format!("#{tag}");
    }
    let c = AnsiStyle::new().fg_color(Some(Color::Ansi(AnsiColor::Magenta)));
    format!("{c}#{tag}{c:#}")
}

/// `text` dimmed (bright-black) when color is enabled, else plain.
fn dim(style: &Style, text: &str) -> String {
    if !style.color {
        return text.to_string();
    }
    let c = AnsiStyle::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));
    format!("{c}{text}{c:#}")
}

fn render_markdown(counts: &[TagCount]) -> String {
    if counts.is_empty() {
        return "# Tags\n\n_No tags found._\n".to_string();
    }
    let mut out = String::from("# Tags\n\n");
    for tc in counts {
        out.push_str(&format!("- #{} ({})\n", tc.tag, tc.count));
    }
    out
}
