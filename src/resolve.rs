//! Fuzzy resolution of a title query to a single memo.
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::cmp::Reverse;

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

use crate::{config::Config, error::StplError, memo::Memo, store};

/// Outcome of matching a query against a list of named things.
#[derive(Debug, PartialEq, Eq)]
pub enum Pick<T> {
    NotFound,
    One(T),
    /// The cluster of close matches, best first.
    Ambiguous(Vec<T>),
}

/// The shared matching algorithm behind [`resolve_one`] and
/// [`crate::diary::resolve_one`].
///
/// `title` and `slug` project the two fields a query may match exactly; only
/// `title` is fuzzy-scored.
///
/// 1. Case-insensitive exact match on title OR slug -> that item.
/// 2. Otherwise fuzzy-score titles with `SkimMatcherV2`, keep positives,
///    sort descending.
/// 3. 0 matches -> `NotFound`. A single match, or a top score beating #2 by at
///    least 1.5x -> `One`. Several close matches -> `Ambiguous`.
pub fn pick<T: Clone>(
    items: &[T],
    query: &str,
    title: impl Fn(&T) -> &str,
    slug: impl Fn(&T) -> &str,
) -> Pick<T> {
    // 1. Case-insensitive exact match on title OR slug.
    let needle = query.to_lowercase();
    if let Some(item) = items
        .iter()
        .find(|i| title(i).to_lowercase() == needle || slug(i).to_lowercase() == needle)
    {
        return Pick::One(item.clone());
    }

    // 2. Fuzzy-score titles; keep positive scores, sort descending.
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &T)> = items
        .iter()
        .filter_map(|i| matcher.fuzzy_match(title(i), query).map(|s| (s, i)))
        .filter(|(s, _)| *s > 0)
        .collect();
    scored.sort_by_key(|s| Reverse(s.0));

    // 3. Decide based on count / margin.
    match scored.len() {
        0 => Pick::NotFound,
        1 => Pick::One(scored[0].1.clone()),
        _ => {
            let top = scored[0].0;
            let second = scored[1].0;
            // A clear top score beats #2 by a comfortable margin: at least
            // 1.5x the runner-up. Otherwise the choice is ambiguous.
            if top as f64 >= second as f64 * 1.5 {
                Pick::One(scored[0].1.clone())
            } else {
                // Return the cluster of close matches (all within the margin
                // of the top score) so the caller can list them.
                let cutoff = top as f64 / 1.5;
                let matches = scored
                    .iter()
                    .filter(|(s, _)| *s as f64 >= cutoff)
                    .map(|(_, i)| (*i).clone())
                    .collect();
                Pick::Ambiguous(matches)
            }
        }
    }
}

/// Resolve a free-form `query` to exactly one memo.
///
/// Loads every memo via `store::list_all`, then applies [`pick`]:
/// an exact (case-insensitive) title or slug match wins, otherwise the fuzzy
/// scores decide between `NotFound`, a single memo, and `Ambiguous`.
///
/// Callers render `Ambiguous.matches` as clickable memo lines.
pub fn resolve_one(config: &Config, query: &str) -> Result<Memo, StplError> {
    let memos = store::list_all(config).map_err(|_| StplError::NotFound(query.to_string()))?;

    match pick(&memos, query, |m| &m.title, |m| &m.slug) {
        Pick::One(memo) => Ok(memo),
        Pick::NotFound => Err(StplError::NotFound(query.to_string())),
        Pick::Ambiguous(matches) => Err(StplError::Ambiguous {
            query: query.to_string(),
            matches,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(title, slug)` pairs, so `pick` is exercised without touching the
    /// filesystem or the `Memo` model.
    fn items() -> Vec<(String, String)> {
        [
            ("Standup Notes", "standup-notes"),
            ("Release Plan", "release-plan"),
            ("Retro", "retro"),
        ]
        .iter()
        .map(|(t, s)| (t.to_string(), s.to_string()))
        .collect()
    }

    fn pick_from(items: &[(String, String)], query: &str) -> Pick<(String, String)> {
        pick(items, query, |i| i.0.as_str(), |i| i.1.as_str())
    }

    #[test]
    fn exact_title_match_wins_case_insensitively() {
        let items = items();
        assert_eq!(
            pick_from(&items, "standup notes"),
            Pick::One(items[0].clone())
        );
    }

    #[test]
    fn exact_slug_match_wins() {
        let items = items();
        assert_eq!(
            pick_from(&items, "release-plan"),
            Pick::One(items[1].clone())
        );
    }

    #[test]
    fn single_fuzzy_match_is_returned() {
        let items = items();
        assert_eq!(pick_from(&items, "retr"), Pick::One(items[2].clone()));
    }

    #[test]
    fn no_match_is_not_found() {
        assert_eq!(pick_from(&items(), "zzzz"), Pick::NotFound);
    }

    #[test]
    fn close_matches_are_ambiguous() {
        let items: Vec<(String, String)> = [("Memo One", "memo-one"), ("Memo Two", "memo-two")]
            .iter()
            .map(|(t, s)| (t.to_string(), s.to_string()))
            .collect();
        match pick_from(&items, "memo") {
            Pick::Ambiguous(matches) => assert_eq!(matches.len(), 2),
            other => panic!("expected ambiguous, got {other:?}"),
        }
    }
}
