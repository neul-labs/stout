//! Brewfile parser
//!
//! Parses Homebrew Brewfile format with two strategies:
//! 1. Ruby parser (full compatibility) - shells out to Ruby
//! 2. Rust parser (fallback) - handles common cases

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tracing::{debug, warn};

/// Parsed Brewfile contents
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Brewfile {
    #[serde(default)]
    pub taps: Vec<TapEntry>,

    #[serde(default)]
    pub brews: Vec<BrewEntry>,

    #[serde(default)]
    pub casks: Vec<CaskEntry>,

    #[serde(default)]
    pub mas: Vec<MasEntry>,

    #[serde(default)]
    pub whalebrew: Vec<WhalebrewEntry>,

    #[serde(default)]
    pub vscode: Vec<VscodeEntry>,
}

/// Tap entry (custom repository)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TapEntry {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub force_auto_update: Option<bool>,
}

/// Brew entry (formula)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BrewEntry {
    pub name: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub link: Option<bool>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    #[serde(default)]
    pub restart_service: Option<RestartService>,
    #[serde(default)]
    pub start_service: Option<bool>,
}

/// Restart service option
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RestartService {
    Bool(bool),
    Symbol(String), // :changed, :immediately
}

/// Cask entry (application)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CaskEntry {
    pub name: String,
    #[serde(default)]
    pub args: CaskArgs,
    #[serde(default)]
    pub greedy: bool,
}

/// Cask installation arguments
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CaskArgs {
    #[serde(default)]
    pub appdir: Option<String>,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub require_sha: bool,
    #[serde(default)]
    pub no_quarantine: bool,
}

/// Mac App Store entry
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MasEntry {
    pub name: String,
    pub id: u64,
}

/// Whalebrew entry (Docker-based CLI tools)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WhalebrewEntry {
    pub name: String,
}

/// VS Code extension entry
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VscodeEntry {
    pub name: String,
}

impl Brewfile {
    /// Parse a Brewfile from the given path
    pub fn parse(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::BrewfileNotFound(path.display().to_string()));
        }

        // Try Ruby parser first (full compatibility)
        match Self::parse_with_ruby(path) {
            Ok(bf) => {
                debug!("Parsed Brewfile with Ruby parser");
                return Ok(bf);
            }
            Err(e) => {
                debug!("Ruby parser failed: {}, trying Rust parser", e);
            }
        }

        // Fall back to Rust parser
        warn!("Ruby not available, using basic Rust parser (some options may be ignored)");
        Self::parse_with_rust(path)
    }

    /// Parse Brewfile using Ruby for full DSL compatibility
    fn parse_with_ruby(path: &Path) -> Result<Self> {
        const RUBY_SCRIPT: &str = r#"
require 'json'

$e = {
  taps: [],
  brews: [],
  casks: [],
  mas: [],
  whalebrew: [],
  vscode: []
}

def tap(name, url: nil, force_auto_update: nil)
  entry = { name: name }
  entry[:url] = url if url
  entry[:force_auto_update] = force_auto_update unless force_auto_update.nil?
  $e[:taps] << entry
end

def brew(name, args: [], link: nil, conflicts_with: [], restart_service: nil, start_service: nil)
  entry = { name: name }
  entry[:args] = args unless args.empty?
  entry[:link] = link unless link.nil?
  entry[:conflicts_with] = conflicts_with unless conflicts_with.empty?
  entry[:restart_service] = restart_service.to_s if restart_service
  entry[:start_service] = start_service unless start_service.nil?
  $e[:brews] << entry
end

def cask(name, args: {}, greedy: false)
  entry = { name: name }
  entry[:args] = args unless args.empty?
  entry[:greedy] = greedy if greedy
  $e[:casks] << entry
end

def mas(name, id:)
  $e[:mas] << { name: name, id: id }
end

def whalebrew(name)
  $e[:whalebrew] << { name: name }
end

def vscode(name)
  $e[:vscode] << { name: name }
end

# Ignore cask_args (global cask settings)
def cask_args(args = {})
end

begin
  eval(File.read(ARGV[0]))
  puts JSON.generate($e)
rescue => e
  STDERR.puts "Error: #{e.message}"
  exit 1
end
"#;

        let output = Command::new("ruby")
            .arg("-e")
            .arg(RUBY_SCRIPT)
            .arg(path)
            .output()
            .map_err(|e| Error::RubyError(format!("Failed to execute Ruby: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::RubyError(stderr.to_string()));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let brewfile: Brewfile = serde_json::from_str(&json_str)
            .map_err(|e| Error::ParseError(format!("Failed to parse Ruby output: {}", e)))?;

        Ok(brewfile)
    }

    /// Parse Brewfile using Rust (handles common cases)
    fn parse_with_rust(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut brewfile = Brewfile::default();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse tap entries
            if let Some(rest) = line.strip_prefix("tap ") {
                if let Some(name) = extract_quoted_string(rest) {
                    brewfile.taps.push(TapEntry {
                        name,
                        ..Default::default()
                    });
                }
                continue;
            }

            // Parse brew entries
            if let Some(rest) = line.strip_prefix("brew ") {
                if let Some(name) = extract_quoted_string(rest) {
                    brewfile.brews.push(BrewEntry {
                        name,
                        ..Default::default()
                    });
                }
                continue;
            }

            // Parse cask entries
            if let Some(rest) = line.strip_prefix("cask ") {
                if let Some(name) = extract_quoted_string(rest) {
                    brewfile.casks.push(CaskEntry {
                        name,
                        ..Default::default()
                    });
                }
                continue;
            }

            // Parse mas entries
            if let Some(rest) = line.strip_prefix("mas ") {
                if let Some((name, id)) = parse_mas_entry(rest) {
                    brewfile.mas.push(MasEntry { name, id });
                }
                continue;
            }

            // Parse whalebrew entries
            if let Some(rest) = line.strip_prefix("whalebrew ") {
                if let Some(name) = extract_quoted_string(rest) {
                    brewfile.whalebrew.push(WhalebrewEntry { name });
                }
                continue;
            }

            // Parse vscode entries
            if let Some(rest) = line.strip_prefix("vscode ") {
                if let Some(name) = extract_quoted_string(rest) {
                    brewfile.vscode.push(VscodeEntry { name });
                }
                continue;
            }
        }

        Ok(brewfile)
    }

    /// Generate a Brewfile from the current state
    pub fn generate(
        taps: &[String],
        formulas: &[(String, bool)], // (name, requested)
        casks: &[String],
    ) -> String {
        let mut output = String::new();

        // Taps
        if !taps.is_empty() {
            output.push_str("# Taps\n");
            for tap in taps {
                output.push_str(&format!("tap \"{}\"\n", tap));
            }
            output.push('\n');
        }

        // Formulas (only requested ones by default)
        let requested: Vec<_> = formulas.iter().filter(|(_, r)| *r).collect();
        if !requested.is_empty() {
            output.push_str("# Formulas\n");
            for (name, _) in requested {
                output.push_str(&format!("brew \"{}\"\n", name));
            }
            output.push('\n');
        }

        // Casks
        if !casks.is_empty() {
            output.push_str("# Casks\n");
            for cask in casks {
                output.push_str(&format!("cask \"{}\"\n", cask));
            }
            output.push('\n');
        }

        output
    }

    /// Check if Brewfile is empty
    pub fn is_empty(&self) -> bool {
        self.taps.is_empty()
            && self.brews.is_empty()
            && self.casks.is_empty()
            && self.mas.is_empty()
            && self.whalebrew.is_empty()
            && self.vscode.is_empty()
    }

    /// Get total entry count
    pub fn entry_count(&self) -> usize {
        self.taps.len()
            + self.brews.len()
            + self.casks.len()
            + self.mas.len()
            + self.whalebrew.len()
            + self.vscode.len()
    }
}

/// Extract a quoted string from the start of a line
fn extract_quoted_string(s: &str) -> Option<String> {
    let s = s.trim();

    // Handle double-quoted strings
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }

    // Handle single-quoted strings
    if let Some(rest) = s.strip_prefix('\'') {
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    None
}

/// Parse a mas entry: mas "Name", id: 123456
fn parse_mas_entry(s: &str) -> Option<(String, u64)> {
    let name = extract_quoted_string(s)?;

    // Find id: after the name
    if let Some(id_pos) = s.find("id:") {
        let id_str = s[id_pos + 3..].trim();
        // Extract digits
        let id_digits: String = id_str.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(id) = id_digits.parse() {
            return Some((name, id));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_quoted_string() {
        assert_eq!(extract_quoted_string("\"jq\""), Some("jq".to_string()));
        assert_eq!(extract_quoted_string("'jq'"), Some("jq".to_string()));
        assert_eq!(
            extract_quoted_string("\"homebrew/cask\""),
            Some("homebrew/cask".to_string())
        );
    }

    #[test]
    fn test_parse_mas_entry() {
        assert_eq!(
            parse_mas_entry("\"Xcode\", id: 497799835"),
            Some(("Xcode".to_string(), 497799835))
        );
    }

    #[test]
    fn test_generate_brewfile() {
        let taps = vec!["homebrew/cask".to_string()];
        let formulas = vec![
            ("jq".to_string(), true),
            ("oniguruma".to_string(), false), // dependency
        ];
        let casks = vec!["firefox".to_string()];

        let output = Brewfile::generate(&taps, &formulas, &casks);

        assert!(output.contains("tap \"homebrew/cask\""));
        assert!(output.contains("brew \"jq\""));
        assert!(!output.contains("brew \"oniguruma\"")); // dependency excluded
        assert!(output.contains("cask \"firefox\""));
    }
}
