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
    pub head: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlSpec {
    pub url: String,
    #[serde(default)]
    pub sha256: Option<String>,
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

        Formula {
            name: hb.name,
            version,
            revision: hb.revision,
            desc: hb.desc,
            homepage: hb.homepage,
            license: hb.license,
            tap: hb.tap,
            urls: FormulaUrls {
                stable: None,
                head: None,
            },
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
