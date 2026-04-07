// SPDX-License-Identifier: MPL-2.0

/// Recursively rename all table keys in a `toml::Value` using `rename_fn`.
pub(super) fn rename_toml_keys(value: toml::Value, rename_fn: fn(&str) -> String) -> toml::Value {
    match value {
        toml::Value::Table(table) => {
            let renamed = table
                .into_iter()
                .map(|(k, v)| (rename_fn(&k), rename_toml_keys(v, rename_fn)))
                .collect();
            toml::Value::Table(renamed)
        }
        toml::Value::Array(arr) => toml::Value::Array(
            arr.into_iter()
                .map(|v| rename_toml_keys(v, rename_fn))
                .collect(),
        ),
        other => other,
    }
}

/// Convert a camelCase identifier to snake_case.
///
/// Examples: `fontSize` → `font_size`, `scrollbackLines` → `scrollback_lines`.
pub(super) fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.char_indices() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// Convert a snake_case identifier to camelCase.
///
/// Examples: `font_size` → `fontSize`, `scrollback_lines` → `scrollbackLines`.
pub(super) fn snake_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut next_upper = false;
    for ch in s.chars() {
        if ch == '_' {
            next_upper = true;
        } else if next_upper {
            out.extend(ch.to_uppercase());
            next_upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}
