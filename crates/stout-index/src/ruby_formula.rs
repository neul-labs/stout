//! Parse Homebrew Ruby formula files to extract metadata for installation
//! from third-party taps. This is a simple regex-free parser that handles
//! the common subset of Homebrew's Ruby DSL.

use crate::error::Result;
use crate::formula::{
    Bottle, Dependencies, Formula, FormulaFlags, FormulaMeta, FormulaUrls, HeadUrlSpec, UrlSpec,
};
use std::collections::HashMap;

/// Parse a Homebrew Ruby formula file and extract key fields into a `Formula`.
///
/// This parser is intentionally limited — it handles the metadata fields
/// commonly needed for installation (version, url, sha256, dependencies,
/// bottle hashes) but does NOT attempt to fully evaluate the Ruby DSL.
/// Conditional blocks (`on_macos`, `on_linux`, `if` statements) are not
/// evaluated; only top-level declarations are captured.
pub fn parse_ruby_formula(content: &str, tap: &str, name: &str) -> Result<Formula> {
    // Strip block bodies (between `do` and `end`) to avoid matching
    // fields inside install, test, or other method blocks.
    let top = strip_block_bodies(content);

    let version = extract_quoted(&top, "version")
        .or_else(|| infer_version_from_url(&top))
        .unwrap_or_else(|| "unknown".to_string());

    // Only take the first url/sha256 that appears OUTSIDE a bottle block
    let stable_url = extract_stable_url(&top);
    let stable_sha256 = extract_stable_sha256(&top);

    let urls = FormulaUrls {
        stable: stable_url.map(|url| UrlSpec {
            sha256: stable_sha256,
            url,
        }),
        head: extract_quoted(&top, "head").map(|url| HeadUrlSpec {
            branch: extract_head_branch(content),
            url,
        }),
    };

    let bottles = extract_bottles(content, name);
    let dependencies = extract_dependencies(&top);
    let conflicts = extract_conflicts(&top);
    let desc = extract_quoted(&top, "desc");
    let homepage = extract_quoted(&top, "homepage");
    let license_val = extract_quoted(&top, "license");
    let revision = extract_revision(&top);
    let caveats = extract_caveats(content);

    Ok(Formula {
        name: name.to_string(),
        version,
        revision,
        desc,
        homepage,
        license: license_val,
        tap: tap.to_string(),
        urls,
        bottles,
        dependencies,
        aliases: vec![],
        conflicts_with: conflicts,
        caveats,
        flags: FormulaFlags {
            keg_only: false,
            deprecated: false,
            disabled: false,
            has_post_install: false,
        },
        service: None,
        meta: FormulaMeta {
            ruby_source_path: None,
            tap_git_head: None,
        },
    })
}

/// Remove content inside `do ... end` blocks and `def ... end` methods
/// to avoid matching fields inside install/test/etc. methods.
fn strip_block_bodies(content: &str) -> String {
    let mut result = String::new();
    let mut depth = 0i32;
    let mut i = 0;
    let bytes = content.as_bytes();

    while i < bytes.len() {
        // Check for `def` keyword (method definition)
        if i + 2 < bytes.len() && &bytes[i..i + 3] == b"def" {
            let before = if i > 0 { bytes[i - 1] } else { b' ' };
            let after = if i + 3 < bytes.len() {
                bytes[i + 3]
            } else {
                b' '
            };
            if is_word_boundary(before) && is_word_boundary(after) {
                depth += 1;
                i += 3;
                continue;
            }
        }

        // Check for `do` keyword (block)
        if i + 1 < bytes.len() && bytes[i] == b'd' && bytes[i + 1] == b'o' {
            // Check it's a standalone "do" word, not part of "depends_on" etc.
            let before = if i > 0 { bytes[i - 1] } else { b' ' };
            let after = if i + 2 < bytes.len() {
                bytes[i + 2]
            } else {
                b' '
            };
            if is_word_boundary(before) && is_word_boundary(after) {
                depth += 1;
                i += 2;
                continue;
            }
        }

        if depth > 0 {
            // Check for matching `end`
            if i + 2 < bytes.len() && &bytes[i..i + 3] == b"end" {
                let before = if i > 0 { bytes[i - 1] } else { b' ' };
                let after = if i + 3 < bytes.len() {
                    bytes[i + 3]
                } else {
                    b' '
                };
                if is_word_boundary(before) && is_word_boundary(after) {
                    depth -= 1;
                    i += 3;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        result.push(bytes[i] as char);
        i += 1;
    }

    result
}

fn is_word_boundary(c: u8) -> bool {
    !c.is_ascii_alphanumeric() && c != b'_'
}

/// Extract a quoted string value for a given key, e.g. `key "value"`.
fn extract_quoted(content: &str, key: &str) -> Option<String> {
    let pattern = format!(r#"{} "#, key);
    let mut search_start = 0;
    loop {
        let pos = content[search_start..].find(&pattern)? + search_start;
        let after_key = pos + pattern.len();
        let remaining = &content[after_key..];

        // Only match outside comments
        let line_before = &content[..pos];
        let last_newline = line_before.rfind('\n').unwrap_or(0);
        let line_prefix = &line_before[last_newline..].trim();
        if line_prefix.starts_with('#') {
            search_start = pos + 1;
            continue;
        }

        if let Some(stripped) = remaining.strip_prefix('"') {
            let end = stripped.find('"')?;
            return Some(stripped[..end].to_string());
        }
        search_start = pos + 1;
    }
}

/// Extract revision number (defaults to 0).
fn extract_revision(content: &str) -> u32 {
    let pattern = "revision ";
    content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return None;
            }
            if let Some(pos) = trimmed.find(pattern) {
                let after = trimmed[pos + pattern.len()..].trim();
                after.split_whitespace().next().and_then(|n| n.parse().ok())
            } else {
                None
            }
        })
        .unwrap_or(0)
}

/// Extract dependencies from top-level `depends_on` declarations.
fn extract_dependencies(content: &str) -> Dependencies {
    let mut runtime = Vec::new();
    let mut build = Vec::new();
    let mut test = Vec::new();
    let mut optional = Vec::new();
    let mut recommended = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.starts_with("depends_on") {
            continue;
        }

        // Extract the dependency name
        let name = match extract_dep_name(trimmed) {
            Some(n) => n,
            None => continue,
        };

        // Skip non-formula deps (:macos, :xcode, etc.)
        if name.starts_with(':') {
            continue;
        }

        if trimmed.contains(":build") {
            build.push(name);
        } else if trimmed.contains(":test") {
            test.push(name);
        } else if trimmed.contains(":optional") {
            optional.push(name);
        } else if trimmed.contains(":recommended") {
            recommended.push(name);
        } else {
            runtime.push(name);
        }
    }

    Dependencies {
        runtime,
        build,
        test,
        optional,
        recommended,
    }
}

/// Extract depends_on name from a line like `depends_on "name" => :build`.
fn extract_dep_name(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract bottle SHA256 hashes from bottle blocks.
fn extract_bottles(content: &str, _name: &str) -> HashMap<String, Bottle> {
    let mut bottles = HashMap::new();

    // Find the bottle block
    let bottle_start = match content.find("\nbottle") {
        Some(pos) => pos,
        None => {
            // Try with "bottle do"
            match content.find("bottle do") {
                Some(pos) => pos,
                None => return bottles,
            }
        }
    };

    let bottle_section = &content[bottle_start..];

    // Find the matching `end`
    let end_pos = find_matching_end(bottle_section).unwrap_or(bottle_section.len());
    let section = &bottle_section[..end_pos];

    // Parse each sha256 line
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("sha256") {
            continue;
        }

        // Format: sha256 cellar: :any, arm64_sequoia: "abc123..."
        // Or: sha256 arm64_sequoia: "abc123..."
        let after_sha256 = trimmed.trim_start_matches("sha256").trim();

        // Find the quote position to identify where the hash is
        if let Some(quote_pos) = after_sha256.find('"') {
            // Find the last colon before the quote (this is platform:)
            if let Some(colon_pos) = after_sha256[..quote_pos].rfind(':') {
                // Get platform name (skip cellar/comma if present)
                let before_colon = &after_sha256[..colon_pos];
                let platform = if let Some(last_comma) = before_colon.rfind(',') {
                    before_colon[last_comma + 1..].trim().to_string()
                } else {
                    before_colon.trim().to_string()
                };

                // Extract the hash between the quotes
                let rest = &after_sha256[colon_pos + 1..].trim();
                if let Some(stripped) = rest.strip_prefix('"') {
                    if let Some(end_quote) = stripped.find('"') {
                        let sha256 = stripped[..end_quote].to_string();
                        bottles.insert(
                            platform,
                            Bottle {
                                url: String::new(),
                                sha256,
                                cellar: "/opt/homebrew/Cellar".to_string(),
                            },
                        );
                    }
                }
            }
        }
    }

    bottles
}

/// Find the position of the `end` matching a `do` block, starting from pos 0.
fn find_matching_end(content: &str) -> Option<usize> {
    let mut depth = 1i32; // start at 1 since we're inside the block
    let mut i = 0;
    let bytes = content.as_bytes();

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'd' && bytes[i + 1] == b'o' {
            let before = if i > 0 { bytes[i - 1] } else { b' ' };
            let after = if i + 2 < bytes.len() {
                bytes[i + 2]
            } else {
                b' '
            };
            if is_word_boundary(before) && is_word_boundary(after) {
                depth += 1;
                i += 2;
                continue;
            }
        }

        if i + 2 < bytes.len() && &bytes[i..i + 3] == b"end" {
            let before = if i > 0 { bytes[i - 1] } else { b' ' };
            let after = if i + 3 < bytes.len() {
                bytes[i + 3]
            } else {
                b' '
            };
            if is_word_boundary(before) && is_word_boundary(after) {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 3);
                }
                i += 3;
                continue;
            }
        }

        i += 1;
    }

    None
}

/// Extract conflicts_with names.
fn extract_conflicts(content: &str) -> Vec<String> {
    let mut conflicts = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("conflicts_with") {
            if let Some(name) = extract_dep_name(trimmed) {
                conflicts.push(name);
            }
        }
    }
    conflicts
}

/// Extract caveats block content.
fn extract_caveats(content: &str) -> Option<String> {
    let caveat_start = content.find("def caveats")?;

    // Find the matching `end` after the caveats block
    let section = &content[caveat_start..];
    let end_pos = find_matching_end(section).unwrap_or(section.len());

    let caveat_section = &section[..end_pos];

    // Extract string literals between quotes
    let mut caveats = Vec::new();
    for line in caveat_section.lines().skip(1) {
        // Skip `end` line and heredoc markers
        let trimmed = line.trim();
        if trimmed == "end" || trimmed.starts_with("<<~") || trimmed == "EOS" {
            continue;
        }
        // Extract quoted strings
        if let Some(content) = extract_caveat_line(trimmed) {
            caveats.push(content);
        }
    }

    if caveats.is_empty() {
        None
    } else {
        Some(caveats.join("\n"))
    }
}

fn extract_caveat_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return None;
    }
    // Handle `<<~EOS` heredoc syntax
    if trimmed.starts_with("<<~") {
        return None;
    }
    // Extract between quotes
    if let Some(start) = trimmed.find('"') {
        let rest = &trimmed[start + 1..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Extract the stable source URL (first url outside bottle block).
fn extract_stable_url(content: &str) -> Option<String> {
    // Remove the bottle block first to avoid matching URLs inside it
    let without_bottle = remove_bottle_block(content);
    extract_quoted(&without_bottle, "url")
}

/// Extract the stable SHA256 (first sha256 outside bottle block).
fn extract_stable_sha256(content: &str) -> Option<String> {
    let without_bottle = remove_bottle_block(content);
    // Find the sha256 that's NOT inside a bottle block
    for line in without_bottle.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("sha256") {
            // Make sure it's a hex string, not a symbol
            if let Some(val) = extract_quoted(trimmed, "sha256") {
                if val.len() == 64 && val.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Remove the bottle block from content to avoid matching its urls/sha256.
fn remove_bottle_block(content: &str) -> String {
    let mut result = String::new();
    let mut in_bottle = false;
    let mut depth = 0i32;
    let mut i = 0;
    let bytes = content.as_bytes();

    while i < bytes.len() {
        if !in_bottle {
            // Check for "bottle do" or "\nbottle"
            let looking_at = if i + 1 < bytes.len() {
                &content[i..]
            } else {
                ""
            };
            if looking_at.starts_with("bottle do") || looking_at.starts_with("\nbottle do") {
                in_bottle = true;
                depth = 1;
                // Skip past "bottle do"
                if looking_at.starts_with("\nbottle do") {
                    i += 11;
                } else {
                    i += 10;
                }
                continue;
            }
            result.push(bytes[i] as char);
            i += 1;
        } else {
            // Track depth inside bottle block
            let looking_at = &content[i..];
            if looking_at.starts_with("do ") || looking_at.starts_with("do\n") {
                depth += 1;
                i += 2;
            } else if looking_at.starts_with("end") {
                let before = if i > 0 { bytes[i - 1] } else { b' ' };
                let after = if i + 3 < bytes.len() {
                    bytes[i + 3]
                } else {
                    b' '
                };
                if is_word_boundary(before) && is_word_boundary(after) {
                    depth -= 1;
                    i += 3;
                    if depth == 0 {
                        in_bottle = false;
                        // Don't add a newline if we already have one
                        if !result.ends_with('\n') {
                            result.push('\n');
                        }
                    }
                    continue;
                }
            }
            i += 1;
        }
    }

    result
}

/// Extract branch from head URL declaration.
/// Format: head "url", branch: "name" or just head "url"
fn extract_head_branch(content: &str) -> Option<String> {
    let head_line = content
        .lines()
        .find(|line| line.trim().starts_with("head "))?;
    let trimmed = head_line.trim();

    if let Some(branch_start) = trimmed.find("branch:") {
        let after = &trimmed[branch_start + 7..].trim();
        if let Some(start) = after.find('"') {
            let rest = &after[start + 1..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// Try to infer version from the stable URL (Homebrew convention).
fn infer_version_from_url(content: &str) -> Option<String> {
    let url = extract_stable_url(content)?;
    // Try common patterns: foo-1.2.3.tar.gz, foo-1.2.3.tgz
    let filename = url.rsplit('/').next()?;
    // Remove common extensions
    let stem = filename
        .strip_suffix(".tar.gz")
        .or_else(|| filename.strip_suffix(".tgz"))
        .or_else(|| filename.strip_suffix(".tar.bz2"))
        .or_else(|| filename.strip_suffix(".tar.xz"))
        .or_else(|| filename.strip_suffix(".tar.lz"))
        .or_else(|| filename.strip_suffix(".zip"))
        .or_else(|| filename.strip_suffix(".tar"))?;

    // Extract version-like suffix after last hyphen
    // e.g., "foo-1.2.3" -> "1.2.3", "openssh-10.3p1" -> "10.3p1"
    if let Some(hyphen) = stem.rfind('-') {
        let candidate = &stem[hyphen + 1..];
        // Must start with a digit
        if candidate.starts_with(|c: char| c.is_ascii_digit()) {
            return Some(candidate.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_quoted() {
        assert_eq!(
            extract_quoted(r#"desc "A package""#, "desc"),
            Some("A package".to_string())
        );
        assert_eq!(
            extract_quoted(r#"homepage "https://example.com""#, "homepage"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            extract_quoted(r#"url "https://example.com/pkg-1.0.tar.gz""#, "url"),
            Some("https://example.com/pkg-1.0.tar.gz".to_string())
        );
    }

    #[test]
    fn test_extract_revision() {
        assert_eq!(extract_revision("revision 1"), 1);
        assert_eq!(extract_revision("revision 0"), 0);
        assert_eq!(
            extract_revision("url \"https://example.com/pkg-1.0.tar.gz\""),
            0
        );
    }

    #[test]
    fn test_strip_block_bodies() {
        let content = r#"
desc "A package"
url "https://example.com/pkg-1.0.tar.gz"

def install
  system "make"
end

depends_on "cmake"
"#;
        let stripped = strip_block_bodies(content);
        assert!(stripped.contains(r#"desc "A package""#));
        assert!(stripped.contains(r#"url "https://example.com/pkg-1.0.tar.gz""#));
        assert!(stripped.contains(r#"depends_on "cmake""#));
        assert!(!stripped.contains("def install"));
        assert!(!stripped.contains(r#"system "make""#));
    }

    #[test]
    fn test_extract_dependencies() {
        let content = r#"
depends_on "cmake" => :build
depends_on "openssl@3"
depends_on "pkgconf" => :build
depends_on "gettext" => :test
"#;
        let deps = extract_dependencies(content);
        assert_eq!(deps.runtime, vec!["openssl@3"]);
        assert_eq!(deps.build, vec!["cmake", "pkgconf"]);
        assert_eq!(deps.test, vec!["gettext"]);
    }

    #[test]
    fn test_extract_stable_sha256() {
        let content = r#"
url "https://example.com/pkg-1.0.tar.gz"
sha256 "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
"#;
        assert_eq!(
            extract_stable_sha256(content),
            Some("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string())
        );
    }

    #[test]
    fn test_extract_bottles() {
        let content = r#"
bottle do
  sha256 cellar: :any, arm64_sequoia: "abc123abc123abc123abc123abc123abc123abc123abc123abc123abc123abc1"
  sha256 cellar: :any, arm64_sonoma: "def456def456def456def456def456def456def456def456def456def456def4"
  sha256 arm64_linux: "ghi789ghi789ghi789ghi789ghi789ghi789ghi789ghi789ghi789ghi789ghi7"
end
"#;
        let bottles = extract_bottles(content, "pkg");
        assert_eq!(bottles.len(), 3);
        assert!(bottles.contains_key("arm64_sequoia"));
        assert!(bottles.contains_key("arm64_sonoma"));
        assert!(bottles.contains_key("arm64_linux"));
    }

    #[test]
    fn test_parse_ruby_formula_basic() {
        let content = r#"
class Newbrew < Formula
  desc "A new brew tool"
  homepage "https://github.com/matt-riley/newbrew"
  url "https://github.com/matt-riley/newbrew/archive/refs/tags/v1.0.0.tar.gz"
  sha256 "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
  license "MIT"
  version "1.0.0"

  depends_on "cmake" => :build
  depends_on "openssl@3"

  def install
    system "cmake", "-S", ".", "-B", "build"
  end

  test do
    assert true
  end
end
"#;
        let formula = parse_ruby_formula(content, "matt-riley/tools", "newbrew").unwrap();
        assert_eq!(formula.name, "newbrew");
        assert_eq!(formula.version, "1.0.0");
        assert_eq!(formula.tap, "matt-riley/tools");
        assert_eq!(formula.desc.as_deref(), Some("A new brew tool"));
        assert_eq!(
            formula.homepage.as_deref(),
            Some("https://github.com/matt-riley/newbrew")
        );
        assert_eq!(formula.license.as_deref(), Some("MIT"));
        assert_eq!(formula.revision, 0);
        assert_eq!(
            formula.urls.stable.as_ref().map(|u| u.url.as_str()),
            Some("https://github.com/matt-riley/newbrew/archive/refs/tags/v1.0.0.tar.gz")
        );
        assert_eq!(formula.dependencies.runtime, vec!["openssl@3"]);
        assert_eq!(formula.dependencies.build, vec!["cmake"]);
    }
}
