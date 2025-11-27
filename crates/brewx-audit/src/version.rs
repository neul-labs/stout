//! Version comparison utilities for vulnerability matching

/// Check if an installed version is affected by a vulnerability
///
/// Returns true if the installed version is potentially affected.
/// Uses conservative matching when version ranges are ambiguous.
pub fn version_affected(
    installed: &str,
    affected_range: Option<&str>,
    fixed_version: Option<&str>,
) -> bool {
    // If we have a fixed version, check if installed is less than fixed
    if let Some(fixed) = fixed_version {
        if !fixed.is_empty() {
            return compare_versions(installed, fixed) == std::cmp::Ordering::Less;
        }
    }

    // If we have an affected range, parse and check
    if let Some(range) = affected_range {
        return version_in_range(installed, range);
    }

    // If no version info available, conservatively assume affected
    true
}

/// Compare two version strings
///
/// Returns Ordering::Less if v1 < v2, Equal if v1 == v2, Greater if v1 > v2
pub fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let parts1 = parse_version(v1);
    let parts2 = parse_version(v2);

    for (p1, p2) in parts1.iter().zip(parts2.iter()) {
        match p1.cmp(p2) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    // Handle remaining parts after common prefix
    // If one version has extra parts, check if they're pre-release indicators
    if parts1.len() != parts2.len() {
        let (longer, shorter_len, is_v1_longer) = if parts1.len() > parts2.len() {
            (&parts1, parts2.len(), true)
        } else {
            (&parts2, parts1.len(), false)
        };

        // Check if the extra parts start with a pre-release
        if let Some(first_extra) = longer.get(shorter_len) {
            if matches!(first_extra, VersionPart::PreRelease(_)) {
                // Pre-release version is less than release version
                // e.g., 1.0-alpha < 1.0
                return if is_v1_longer {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
        }

        // Otherwise, longer version is greater (e.g., 1.0.1 > 1.0)
        return if is_v1_longer {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }

    Ordering::Equal
}

/// Parse a version string into comparable parts
fn parse_version(v: &str) -> Vec<VersionPart> {
    let mut parts = Vec::new();

    // Remove common prefixes
    let v = v.trim_start_matches('v').trim_start_matches('V');

    // Split on common separators
    for segment in v.split(|c: char| c == '.' || c == '-' || c == '_' || c == '+') {
        if segment.is_empty() {
            continue;
        }

        // Try to parse as number first
        if let Ok(num) = segment.parse::<u64>() {
            parts.push(VersionPart::Number(num));
        } else {
            // Check for pre-release indicators
            let lower = segment.to_lowercase();
            if lower.starts_with("alpha") || lower.starts_with("a") {
                parts.push(VersionPart::PreRelease(PreRelease::Alpha));
                // Extract trailing number if present
                if let Some(num_str) = lower.strip_prefix("alpha") {
                    if let Ok(num) = num_str.parse::<u64>() {
                        parts.push(VersionPart::Number(num));
                    }
                }
            } else if lower.starts_with("beta") || lower.starts_with("b") {
                parts.push(VersionPart::PreRelease(PreRelease::Beta));
            } else if lower.starts_with("rc") || lower.starts_with("pre") {
                parts.push(VersionPart::PreRelease(PreRelease::RC));
            } else if lower == "dev" || lower == "snapshot" {
                parts.push(VersionPart::PreRelease(PreRelease::Dev));
            } else {
                // Generic string part
                parts.push(VersionPart::String(segment.to_string()));
            }
        }
    }

    parts
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionPart {
    Number(u64),
    PreRelease(PreRelease),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PreRelease {
    Dev,    // dev, snapshot (lowest)
    Alpha,  // alpha, a
    Beta,   // beta, b
    RC,     // rc, pre
}

impl PartialOrd for VersionPart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionPart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        use VersionPart::*;

        match (self, other) {
            (Number(a), Number(b)) => a.cmp(b),
            (PreRelease(a), PreRelease(b)) => a.cmp(b),
            (String(a), String(b)) => a.cmp(b),

            // Numbers are greater than pre-releases
            (Number(_), PreRelease(_)) => Ordering::Greater,
            (PreRelease(_), Number(_)) => Ordering::Less,

            // Strings are compared with numbers lexicographically
            (Number(n), String(s)) => n.to_string().cmp(s),
            (String(s), Number(n)) => s.cmp(&n.to_string()),

            // Pre-releases are less than strings
            (PreRelease(_), String(_)) => Ordering::Less,
            (String(_), PreRelease(_)) => Ordering::Greater,
        }
    }
}

/// Check if a version is in a given range expression
fn version_in_range(version: &str, range: &str) -> bool {
    // Handle common range formats:
    // - ">=1.0, <2.0"
    // - ">=1.0; <2.0"
    // - "1.0, 1.1, 1.2" (explicit versions)
    // - ">= 1.0"
    // - "< 2.0"
    // - "<= 1.5"

    let constraints: Vec<&str> = range
        .split(|c| c == ',' || c == ';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if constraints.is_empty() {
        return true; // No constraints means affected
    }

    // Check if it looks like explicit versions (no operators)
    let has_operators = constraints.iter().any(|c| {
        c.starts_with(">=") || c.starts_with("<=") || c.starts_with('>') || c.starts_with('<')
    });

    if !has_operators {
        // Explicit version list - check if our version matches any
        return constraints.iter().any(|c| {
            compare_versions(version, c) == std::cmp::Ordering::Equal
        });
    }

    // Range constraints - all must be satisfied
    for constraint in constraints {
        let constraint = constraint.trim();

        if let Some(v) = constraint.strip_prefix(">=") {
            let v = v.trim();
            if compare_versions(version, v) == std::cmp::Ordering::Less {
                return false;
            }
        } else if let Some(v) = constraint.strip_prefix("<=") {
            let v = v.trim();
            if compare_versions(version, v) == std::cmp::Ordering::Greater {
                return false;
            }
        } else if let Some(v) = constraint.strip_prefix('>') {
            let v = v.trim();
            if compare_versions(version, v) != std::cmp::Ordering::Greater {
                return false;
            }
        } else if let Some(v) = constraint.strip_prefix('<') {
            let v = v.trim();
            if compare_versions(version, v) != std::cmp::Ordering::Less {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        use std::cmp::Ordering::*;

        assert_eq!(compare_versions("1.0", "2.0"), Less);
        assert_eq!(compare_versions("2.0", "1.0"), Greater);
        assert_eq!(compare_versions("1.0", "1.0"), Equal);

        assert_eq!(compare_versions("1.0.0", "1.0.1"), Less);
        assert_eq!(compare_versions("1.0.10", "1.0.2"), Greater);
        assert_eq!(compare_versions("1.0", "1.0.0"), Less);

        assert_eq!(compare_versions("1.0-alpha", "1.0"), Less);
        assert_eq!(compare_versions("1.0-beta", "1.0-alpha"), Greater);
        assert_eq!(compare_versions("1.0-rc1", "1.0-beta"), Greater);

        assert_eq!(compare_versions("v1.0", "1.0"), Equal);
    }

    #[test]
    fn test_version_in_range() {
        assert!(version_in_range("1.5", ">=1.0, <2.0"));
        assert!(!version_in_range("0.5", ">=1.0, <2.0"));
        assert!(!version_in_range("2.5", ">=1.0, <2.0"));

        assert!(version_in_range("1.0", "1.0, 1.1, 1.2"));
        assert!(version_in_range("1.2", "1.0, 1.1, 1.2"));
        assert!(!version_in_range("1.3", "1.0, 1.1, 1.2"));
    }

    #[test]
    fn test_version_affected() {
        // Fixed version specified
        assert!(version_affected("1.0", None, Some("1.5")));
        assert!(!version_affected("2.0", None, Some("1.5")));

        // Range specified
        assert!(version_affected("1.5", Some(">=1.0, <2.0"), None));
        assert!(!version_affected("2.5", Some(">=1.0, <2.0"), None));

        // Both specified - fixed takes precedence
        assert!(!version_affected("2.0", Some(">=1.0, <2.0"), Some("1.5")));
    }
}
