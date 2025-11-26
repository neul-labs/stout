//! Installation plan generation

use crate::error::Result;
use crate::graph::DependencyGraph;
use brewx_index::FormulaInfo;
use std::collections::HashSet;

/// A single step in an installation plan
#[derive(Debug, Clone)]
pub struct InstallStep {
    pub name: String,
    pub version: String,
    pub is_dependency: bool,
}

/// A complete installation plan
#[derive(Debug)]
pub struct InstallPlan {
    /// Steps to execute in order
    pub steps: Vec<InstallStep>,
    /// Packages explicitly requested
    pub requested: HashSet<String>,
    /// Packages that are dependencies
    pub dependencies: HashSet<String>,
    /// Packages already installed (will be skipped)
    pub already_installed: HashSet<String>,
}

impl InstallPlan {
    /// Create a new empty plan
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            requested: HashSet::new(),
            dependencies: HashSet::new(),
            already_installed: HashSet::new(),
        }
    }

    /// Create a plan from a dependency graph
    pub fn from_graph(
        graph: &DependencyGraph,
        requested: &[&str],
        get_info: impl Fn(&str) -> Option<FormulaInfo>,
        is_installed: impl Fn(&str) -> bool,
    ) -> Result<Self> {
        let mut plan = Self::new();
        plan.requested = requested.iter().map(|s| s.to_string()).collect();

        // Get topological order
        let order = graph.topological_sort()?;

        for name in order {
            if is_installed(&name) {
                plan.already_installed.insert(name.clone());
                continue;
            }

            let is_dep = !plan.requested.contains(&name);
            if is_dep {
                plan.dependencies.insert(name.clone());
            }

            if let Some(info) = get_info(&name) {
                plan.steps.push(InstallStep {
                    name: name.clone(),
                    version: info.version,
                    is_dependency: is_dep,
                });
            }
        }

        Ok(plan)
    }

    /// Get the total number of packages to install
    pub fn total_packages(&self) -> usize {
        self.steps.len()
    }

    /// Check if the plan is empty
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Get packages that will be newly installed
    pub fn new_packages(&self) -> impl Iterator<Item = &InstallStep> {
        self.steps.iter()
    }
}

impl Default for InstallPlan {
    fn default() -> Self {
        Self::new()
    }
}
