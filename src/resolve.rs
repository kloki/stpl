//! Fuzzy resolution of a title query to a single memo.
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::cmp::Reverse;

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

use crate::{config::Config, error::StplError, memo::Memo, store};

/// Resolve a free-form `query` to exactly one memo.
///
/// Algorithm:
/// 1. Load all memos via `store::list_all`.
/// 2. Case-insensitive exact match on title OR slug -> return it.
/// 3. Otherwise fuzzy-score titles with `SkimMatcherV2`; keep positives,
///    sort by score descending.
/// 4. 0 matches  -> `StplError::NotFound`.
///    1 match, or a clear top score beating #2 by a margin -> return it.
///    Several close matches -> `StplError::Ambiguous { query, matches }`.
///
/// Callers render `Ambiguous.matches` as clickable memo lines.
pub fn resolve_one(config: &Config, query: &str) -> Result<Memo, StplError> {
    let memos = store::list_all(config).map_err(|_| StplError::NotFound(query.to_string()))?;

    // 1+2. Case-insensitive exact match on title OR slug.
    let needle = query.to_lowercase();
    if let Some(memo) = memos
        .iter()
        .find(|m| m.title.to_lowercase() == needle || m.slug.to_lowercase() == needle)
    {
        return Ok(memo.clone());
    }

    // 3. Fuzzy-score titles; keep positive scores, sort descending.
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &Memo)> = memos
        .iter()
        .filter_map(|m| matcher.fuzzy_match(&m.title, query).map(|s| (s, m)))
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|s| Reverse(s.0));

    // 4. Decide based on count / margin.
    match scored.len() {
        0 => Err(StplError::NotFound(query.to_string())),
        1 => Ok(scored[0].1.clone()),
        _ => {
            let top = scored[0].0;
            let second = scored[1].0;
            // A clear top score beats #2 by a comfortable margin: at least
            // 1.5x the runner-up. Otherwise the choice is ambiguous.
            if top as f64 >= second as f64 * 1.5 {
                Ok(scored[0].1.clone())
            } else {
                // Return the cluster of close matches (all within the margin
                // of the top score) so the caller can list them.
                let cutoff = top as f64 / 1.5;
                let matches = scored
                    .iter()
                    .filter(|(s, _)| *s as f64 >= cutoff)
                    .map(|(_, m)| (*m).clone())
                    .collect();
                Err(StplError::Ambiguous {
                    query: query.to_string(),
                    matches,
                })
            }
        }
    }
}
