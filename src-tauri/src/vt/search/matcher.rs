// SPDX-License-Identifier: MPL-2.0

use super::api::SearchQuery;

pub(super) enum Matcher {
    Literal {
        needle: String,
        case_sensitive: bool,
    },
    Regex(regex::Regex),
}

pub(super) fn build_matcher(query: &SearchQuery) -> Result<Matcher, regex::Error> {
    if query.regex {
        let pattern = if query.case_sensitive {
            query.text.clone()
        } else {
            format!("(?i){}", query.text)
        };
        Ok(Matcher::Regex(regex::Regex::new(&pattern)?))
    } else {
        Ok(Matcher::Literal {
            needle: if query.case_sensitive {
                query.text.clone()
            } else {
                query.text.to_lowercase()
            },
            case_sensitive: query.case_sensitive,
        })
    }
}
