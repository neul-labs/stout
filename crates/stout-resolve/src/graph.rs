//! Dependency graph construction

use crate::error::{Error, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use stout_index::{Database, DependencyType};

/// A dependency graph for resolution
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Forward edges: package -> dependencies
    edges: HashMap<String, Vec<String>>,
    /// Reverse edges: package -> dependents
    reverse: HashMap<String, Vec<String>>,
    /// All nodes in the graph
    nodes: HashSet<String>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a package to the graph
    pub fn add_node(&mut self, name: &str) {
        self.nodes.insert(name.to_string());
        self.edges.entry(name.to_string()).or_default();
        self.reverse.entry(name.to_string()).or_default();
    }

    /// Add a dependency edge
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.add_node(from);
        self.add_node(to);

        self.edges
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());

        self.reverse
            .entry(to.to_string())
            .or_default()
            .push(from.to_string());
    }

    /// Get dependencies of a package
    pub fn dependencies(&self, name: &str) -> &[String] {
        self.edges.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get dependents of a package (reverse deps)
    pub fn dependents(&self, name: &str) -> &[String] {
        self.reverse.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Check if the graph contains a package
    pub fn contains(&self, name: &str) -> bool {
        self.nodes.contains(name)
    }

    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &String> {
        self.nodes.iter()
    }

    /// Topological sort (returns install order)
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

        // Calculate in-degrees
        for node in &self.nodes {
            in_degree.insert(node, 0);
        }
        for deps in self.edges.values() {
            for dep in deps {
                *in_degree.entry(dep).or_insert(0) += 1;
            }
        }

        // Find nodes with no dependencies
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&node, _)| node)
            .collect();

        while let Some(node) = queue.pop_front() {
            result.push(node.to_string());

            for dep in self.dependencies(node) {
                if let Some(deg) = in_degree.get_mut(dep.as_str()) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        // Check for cycles
        if result.len() != self.nodes.len() {
            let remaining: Vec<_> = self
                .nodes
                .iter()
                .filter(|n| !result.contains(n))
                .cloned()
                .collect();
            return Err(Error::CycleDetected(remaining.join(", ")));
        }

        // Reverse to get install order (deps first)
        result.reverse();
        Ok(result)
    }

    /// Build a dependency graph from the database
    pub fn build_from_db(db: &Database, roots: &[&str], include_build_deps: bool) -> Result<Self> {
        let mut graph = Self::new();
        let mut to_process: VecDeque<String> = roots.iter().map(|s| s.to_string()).collect();
        let mut seen: HashSet<String> = HashSet::new();

        while let Some(name) = to_process.pop_front() {
            if seen.contains(&name) {
                continue;
            }
            seen.insert(name.clone());
            graph.add_node(&name);

            let deps = db.get_dependencies(&name)?;
            for dep in deps {
                match dep.dep_type {
                    DependencyType::Runtime | DependencyType::Recommended => {
                        graph.add_edge(&name, &dep.name);
                        if !seen.contains(&dep.name) {
                            to_process.push_back(dep.name);
                        }
                    }
                    DependencyType::Build if include_build_deps => {
                        graph.add_edge(&name, &dep.name);
                        if !seen.contains(&dep.name) {
                            to_process.push_back(dep.name);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("wget", "openssl");
        graph.add_edge("wget", "libidn2");
        graph.add_edge("openssl", "ca-certificates");

        let order = graph.topological_sort().unwrap();

        // Dependencies should come before dependents
        let wget_pos = order.iter().position(|x| x == "wget").unwrap();
        let openssl_pos = order.iter().position(|x| x == "openssl").unwrap();
        let ca_pos = order.iter().position(|x| x == "ca-certificates").unwrap();

        assert!(ca_pos < openssl_pos);
        assert!(openssl_pos < wget_pos);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("a", "b");
        graph.add_edge("b", "c");
        graph.add_edge("c", "a"); // cycle!

        let result = graph.topological_sort();
        assert!(matches!(result, Err(Error::CycleDetected(_))));
    }
}
