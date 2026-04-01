//! Formula types and structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Basic formula info stored in the SQLite index (fast queries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaInfo {
    pub name: String,
    pub version: String,
    pub revision: u32,
    pub desc: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub tap: String,
    pub deprecated: bool,
    pub disabled: bool,
    pub has_bottle: bool,
    pub json_hash: Option<String>,
}

/// Full formula data from individual JSON files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Formula {
    pub name: String,
    pub version: String,
    pub revision: u32,
    pub desc: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub tap: String,

    #[serde(default)]
    pub urls: FormulaUrls,

    #[serde(default)]
    pub bottles: HashMap<String, Bottle>,

    #[serde(default)]
    pub dependencies: Dependencies,

    #[serde(default)]
    pub aliases: Vec<String>,

    #[serde(default)]
    pub conflicts_with: Vec<String>,

    pub caveats: Option<String>,

    #[serde(default)]
    pub flags: FormulaFlags,

    pub service: Option<serde_json::Value>,

    #[serde(default)]
    pub meta: FormulaMeta,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormulaUrls {
    pub stable: Option<UrlSpec>,
    #[serde(deserialize_with = "deserialize_head_url_spec", default)]
    pub head: Option<HeadUrlSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlSpec {
    pub url: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// HEAD URL specification for building from git
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadUrlSpec {
    /// Git repository URL
    pub url: String,
    /// Branch name (default: "master" or "main")
    #[serde(default)]
    pub branch: Option<String>,
}

/// Helper type for deserializing head URL that can be either a string or object
#[derive(Deserialize)]
#[serde(untagged)]
enum HeadUrlRaw {
    String(String),
    Object {
        url: String,
        #[serde(default)]
        branch: Option<String>,
    },
}

/// Custom deserializer for HeadUrlSpec that handles both string and object formats
fn deserialize_head_url_spec<'de, D>(deserializer: D) -> Result<Option<HeadUrlSpec>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Option<HeadUrlRaw> = Option::deserialize(deserializer)?;
    Ok(raw.map(|r| match r {
        HeadUrlRaw::String(url) => HeadUrlSpec { url, branch: None },
        HeadUrlRaw::Object { url, branch } => HeadUrlSpec { url, branch },
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottle {
    pub url: String,
    pub sha256: String,
    #[serde(default = "default_cellar")]
    pub cellar: String,
}

fn default_cellar() -> String {
    "/opt/homebrew/Cellar".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dependencies {
    #[serde(default)]
    pub runtime: Vec<String>,
    #[serde(default)]
    pub build: Vec<String>,
    #[serde(default)]
    pub test: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
    #[serde(default)]
    pub recommended: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Runtime,
    Build,
    Test,
    Optional,
    Recommended,
}

impl DependencyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Build => "build",
            Self::Test => "test",
            Self::Optional => "optional",
            Self::Recommended => "recommended",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub dep_type: DependencyType,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormulaFlags {
    #[serde(default)]
    pub keg_only: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub has_post_install: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormulaMeta {
    pub ruby_source_path: Option<String>,
    pub tap_git_head: Option<String>,
}

impl Formula {
    /// Get the bottle for the current platform
    ///
    /// Falls back to "all" platform if no platform-specific bottle is found.
    pub fn bottle_for_platform(&self, platform: &str) -> Option<&Bottle> {
        self.bottles
            .get(platform)
            .or_else(|| self.bottles.get("all"))
    }

    /// Get all runtime dependencies
    pub fn runtime_deps(&self) -> &[String] {
        &self.dependencies.runtime
    }

    /// Get all build dependencies
    pub fn build_deps(&self) -> &[String] {
        &self.dependencies.build
    }

    /// Get all test dependencies
    pub fn test_deps(&self) -> &[String] {
        &self.dependencies.test
    }

    /// Get all optional dependencies
    pub fn optional_deps(&self) -> &[String] {
        &self.dependencies.optional
    }

    /// Get all recommended dependencies
    pub fn recommended_deps(&self) -> &[String] {
        &self.dependencies.recommended
    }

    /// Check if this formula has a bottle for any platform
    pub fn has_any_bottle(&self) -> bool {
        !self.bottles.is_empty()
    }
}

/// Homebrew API response format (for fallback fetching)
/// See: https://formulae.brew.sh/api/formula/<name>.json
#[derive(Debug, Clone, Deserialize)]
pub struct HomebrewFormula {
    pub name: String,
    pub full_name: String,
    pub tap: String,
    pub desc: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub revision: u32,
    pub versions: HomebrewVersions,
    #[serde(default)]
    pub urls: HomebrewUrls,
    pub bottle: HomebrewBottle,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub build_dependencies: Vec<String>,
    #[serde(default)]
    pub test_dependencies: Vec<String>,
    #[serde(default)]
    pub recommended_dependencies: Vec<String>,
    #[serde(default)]
    pub optional_dependencies: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    pub caveats: Option<String>,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub keg_only: bool,
    #[serde(default)]
    pub post_install_defined: bool,
    pub service: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HomebrewUrls {
    #[serde(default)]
    pub stable: Option<HomebrewStableUrl>,
    #[serde(default)]
    pub head: Option<HomebrewHeadUrl>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomebrewStableUrl {
    pub url: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomebrewHeadUrl {
    pub url: String,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomebrewVersions {
    pub stable: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomebrewBottle {
    pub stable: Option<HomebrewBottleStable>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomebrewBottleStable {
    pub files: HashMap<String, HomebrewBottleFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomebrewBottleFile {
    pub url: String,
    pub sha256: String,
    #[serde(default = "default_cellar")]
    pub cellar: String,
}

impl From<HomebrewFormula> for Formula {
    fn from(hb: HomebrewFormula) -> Self {
        let version = hb.versions.stable.unwrap_or_else(|| "unknown".to_string());

        let bottles: HashMap<String, Bottle> = hb
            .bottle
            .stable
            .map(|stable| {
                stable
                    .files
                    .into_iter()
                    .map(|(platform, file)| {
                        (
                            platform,
                            Bottle {
                                url: file.url,
                                sha256: file.sha256,
                                cellar: file.cellar,
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Parse URLs from Homebrew API
        let urls = FormulaUrls {
            stable: hb.urls.stable.map(|s| UrlSpec {
                url: s.url,
                sha256: s.sha256,
            }),
            head: hb.urls.head.map(|h| HeadUrlSpec {
                url: h.url,
                branch: h.branch,
            }),
        };

        Formula {
            name: hb.name,
            version,
            revision: hb.revision,
            desc: hb.desc,
            homepage: hb.homepage,
            license: hb.license,
            tap: hb.tap,
            urls,
            bottles,
            dependencies: Dependencies {
                runtime: hb.dependencies,
                build: hb.build_dependencies,
                test: hb.test_dependencies,
                optional: hb.optional_dependencies,
                recommended: hb.recommended_dependencies,
            },
            aliases: hb.aliases,
            conflicts_with: hb.conflicts_with,
            caveats: hb.caveats,
            flags: FormulaFlags {
                keg_only: hb.keg_only,
                deprecated: hb.deprecated,
                disabled: hb.disabled,
                has_post_install: hb.post_install_defined,
            },
            service: hb.service,
            meta: FormulaMeta::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_head_url_as_string() {
        let json = r#"{
            "stable": {
                "url": "https://example.com/stable.tar.gz",
                "sha256": "abc123"
            },
            "head": "https://github.com/example/repo.git"
        }"#;

        let urls: FormulaUrls = serde_json::from_str(json).unwrap();
        assert!(urls.stable.is_some());
        assert!(urls.head.is_some());
        let head = urls.head.unwrap();
        assert_eq!(head.url, "https://github.com/example/repo.git");
        assert!(head.branch.is_none());
    }

    #[test]
    fn test_parse_head_url_as_object() {
        let json = r#"{
            "stable": {
                "url": "https://example.com/stable.tar.gz",
                "sha256": "abc123"
            },
            "head": {
                "url": "https://github.com/example/repo.git",
                "branch": "main"
            }
        }"#;

        let urls: FormulaUrls = serde_json::from_str(json).unwrap();
        assert!(urls.stable.is_some());
        assert!(urls.head.is_some());
        let head = urls.head.unwrap();
        assert_eq!(head.url, "https://github.com/example/repo.git");
        assert_eq!(head.branch, Some("main".to_string()));
    }

    #[test]
    fn test_parse_head_url_null() {
        let json = r#"{
            "stable": {
                "url": "https://example.com/stable.tar.gz",
                "sha256": "abc123"
            },
            "head": null
        }"#;

        let urls: FormulaUrls = serde_json::from_str(json).unwrap();
        assert!(urls.stable.is_some());
        assert!(urls.head.is_none());
    }

    #[test]
    fn test_parse_head_url_missing() {
        let json = r#"{
            "stable": {
                "url": "https://example.com/stable.tar.gz",
                "sha256": "abc123"
            }
        }"#;

        let urls: FormulaUrls = serde_json::from_str(json).unwrap();
        assert!(urls.stable.is_some());
        assert!(urls.head.is_none());
    }
}
