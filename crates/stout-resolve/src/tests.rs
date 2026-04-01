//! Tests for stout-resolve

use crate::error::Error;
use crate::graph::DependencyGraph;
use crate::plan::{InstallPlan, InstallStep};
use std::collections::HashSet;
use stout_index::FormulaInfo;

// ============================================================================
// DependencyGraph tests
// ============================================================================

#[test]
fn test_graph_new_is_empty() {
    let graph = DependencyGraph::new();
    assert_eq!(graph.nodes().count(), 0);
}

#[test]
fn test_graph_add_node() {
    let mut graph = DependencyGraph::new();
    graph.add_node("wget");

    assert!(graph.contains("wget"));
    assert!(!graph.contains("curl"));
    assert_eq!(graph.nodes().count(), 1);
}

#[test]
fn test_graph_add_edge_creates_nodes() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("wget", "openssl");

    assert!(graph.contains("wget"));
    assert!(graph.contains("openssl"));
    assert_eq!(graph.nodes().count(), 2);
}

#[test]
fn test_graph_dependencies() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("wget", "openssl");
    graph.add_edge("wget", "libidn2");

    let deps = graph.dependencies("wget");
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&"openssl".to_string()));
    assert!(deps.contains(&"libidn2".to_string()));

    // openssl has no deps
    assert!(graph.dependencies("openssl").is_empty());
}

#[test]
fn test_graph_dependents() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("wget", "openssl");
    graph.add_edge("curl", "openssl");

    let dependents = graph.dependents("openssl");
    assert_eq!(dependents.len(), 2);
    assert!(dependents.contains(&"wget".to_string()));
    assert!(dependents.contains(&"curl".to_string()));

    // wget has no dependents
    assert!(graph.dependents("wget").is_empty());
}

#[test]
fn test_graph_nonexistent_node_deps() {
    let graph = DependencyGraph::new();

    // Should return empty slice for nonexistent nodes
    assert!(graph.dependencies("nonexistent").is_empty());
    assert!(graph.dependents("nonexistent").is_empty());
}

#[test]
fn test_topological_sort_single_node() {
    let mut graph = DependencyGraph::new();
    graph.add_node("wget");

    let order = graph.topological_sort().unwrap();
    assert_eq!(order, vec!["wget"]);
}

#[test]
fn test_topological_sort_empty_graph() {
    let graph = DependencyGraph::new();
    let order = graph.topological_sort().unwrap();
    assert!(order.is_empty());
}

#[test]
fn test_topological_sort_linear_chain() {
    let mut graph = DependencyGraph::new();
    // a -> b -> c -> d
    graph.add_edge("a", "b");
    graph.add_edge("b", "c");
    graph.add_edge("c", "d");

    let order = graph.topological_sort().unwrap();

    // d should come first, then c, then b, then a
    let a_pos = order.iter().position(|x| x == "a").unwrap();
    let b_pos = order.iter().position(|x| x == "b").unwrap();
    let c_pos = order.iter().position(|x| x == "c").unwrap();
    let d_pos = order.iter().position(|x| x == "d").unwrap();

    assert!(d_pos < c_pos);
    assert!(c_pos < b_pos);
    assert!(b_pos < a_pos);
}

#[test]
fn test_topological_sort_diamond() {
    let mut graph = DependencyGraph::new();
    // Diamond shape: a -> b, a -> c, b -> d, c -> d
    graph.add_edge("a", "b");
    graph.add_edge("a", "c");
    graph.add_edge("b", "d");
    graph.add_edge("c", "d");

    let order = graph.topological_sort().unwrap();

    let a_pos = order.iter().position(|x| x == "a").unwrap();
    let b_pos = order.iter().position(|x| x == "b").unwrap();
    let c_pos = order.iter().position(|x| x == "c").unwrap();
    let d_pos = order.iter().position(|x| x == "d").unwrap();

    // d must come before b and c, which must come before a
    assert!(d_pos < b_pos);
    assert!(d_pos < c_pos);
    assert!(b_pos < a_pos);
    assert!(c_pos < a_pos);
}

#[test]
fn test_cycle_detection_self_loop() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("a", "a"); // self-loop

    let result = graph.topological_sort();
    assert!(matches!(result, Err(Error::CycleDetected(_))));
}

#[test]
fn test_cycle_detection_two_node_cycle() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("a", "b");
    graph.add_edge("b", "a");

    let result = graph.topological_sort();
    assert!(matches!(result, Err(Error::CycleDetected(_))));
}

#[test]
fn test_cycle_detection_partial_cycle() {
    let mut graph = DependencyGraph::new();
    // Some nodes form a cycle, others don't
    graph.add_edge("root", "a");
    graph.add_edge("a", "b");
    graph.add_edge("b", "c");
    graph.add_edge("c", "a"); // cycle: a -> b -> c -> a

    let result = graph.topological_sort();
    assert!(matches!(result, Err(Error::CycleDetected(_))));
}

// ============================================================================
// InstallPlan tests
// ============================================================================

fn make_formula_info(name: &str, version: &str) -> FormulaInfo {
    FormulaInfo {
        name: name.to_string(),
        version: version.to_string(),
        revision: 0,
        desc: None,
        homepage: None,
        license: None,
        tap: "homebrew/core".to_string(),
        deprecated: false,
        disabled: false,
        has_bottle: true,
        json_hash: None,
    }
}

#[test]
fn test_install_plan_new() {
    let plan = InstallPlan::new();
    assert!(plan.is_empty());
    assert_eq!(plan.total_packages(), 0);
    assert!(plan.requested.is_empty());
    assert!(plan.dependencies.is_empty());
    assert!(plan.already_installed.is_empty());
}

#[test]
fn test_install_plan_single_package() {
    let mut graph = DependencyGraph::new();
    graph.add_node("wget");

    let plan = InstallPlan::from_graph(
        &graph,
        &["wget"],
        |name| Some(make_formula_info(name, "1.0.0")),
        |_| false, // nothing installed
    )
    .unwrap();

    assert_eq!(plan.total_packages(), 1);
    assert!(plan.requested.contains("wget"));
    assert!(plan.dependencies.is_empty());
    assert_eq!(plan.steps[0].name, "wget");
    assert!(!plan.steps[0].is_dependency);
}

#[test]
fn test_install_plan_with_deps() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("wget", "openssl");
    graph.add_edge("wget", "libidn2");

    let plan = InstallPlan::from_graph(
        &graph,
        &["wget"],
        |name| Some(make_formula_info(name, "1.0.0")),
        |_| false,
    )
    .unwrap();

    assert_eq!(plan.total_packages(), 3);
    assert!(plan.requested.contains("wget"));
    assert!(plan.dependencies.contains("openssl"));
    assert!(plan.dependencies.contains("libidn2"));

    // Verify install order (deps first)
    let wget_pos = plan.steps.iter().position(|s| s.name == "wget").unwrap();
    let openssl_pos = plan.steps.iter().position(|s| s.name == "openssl").unwrap();
    let libidn_pos = plan.steps.iter().position(|s| s.name == "libidn2").unwrap();

    assert!(openssl_pos < wget_pos);
    assert!(libidn_pos < wget_pos);
}

#[test]
fn test_install_plan_skips_installed() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("wget", "openssl");

    let installed: HashSet<String> = ["openssl"].iter().map(|s| s.to_string()).collect();

    let plan = InstallPlan::from_graph(
        &graph,
        &["wget"],
        |name| Some(make_formula_info(name, "1.0.0")),
        |name| installed.contains(name),
    )
    .unwrap();

    // Only wget should be in steps, openssl is skipped
    assert_eq!(plan.total_packages(), 1);
    assert_eq!(plan.steps[0].name, "wget");
    assert!(plan.already_installed.contains("openssl"));
}

#[test]
fn test_install_plan_multiple_requested() {
    let mut graph = DependencyGraph::new();
    graph.add_node("wget");
    graph.add_node("curl");
    graph.add_edge("curl", "openssl");

    let plan = InstallPlan::from_graph(
        &graph,
        &["wget", "curl"],
        |name| Some(make_formula_info(name, "1.0.0")),
        |_| false,
    )
    .unwrap();

    assert!(plan.requested.contains("wget"));
    assert!(plan.requested.contains("curl"));
    assert!(plan.dependencies.contains("openssl"));

    // wget and curl are not dependencies
    let wget_step = plan.steps.iter().find(|s| s.name == "wget").unwrap();
    let curl_step = plan.steps.iter().find(|s| s.name == "curl").unwrap();
    assert!(!wget_step.is_dependency);
    assert!(!curl_step.is_dependency);
}

#[test]
fn test_install_plan_all_installed() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("wget", "openssl");

    let plan = InstallPlan::from_graph(
        &graph,
        &["wget"],
        |name| Some(make_formula_info(name, "1.0.0")),
        |_| true, // everything installed
    )
    .unwrap();

    assert!(plan.is_empty());
    assert!(plan.already_installed.contains("wget"));
    assert!(plan.already_installed.contains("openssl"));
}

#[test]
fn test_install_plan_preserves_version() {
    let mut graph = DependencyGraph::new();
    graph.add_node("wget");

    let plan = InstallPlan::from_graph(
        &graph,
        &["wget"],
        |_| Some(make_formula_info("wget", "1.24.5")),
        |_| false,
    )
    .unwrap();

    assert_eq!(plan.steps[0].version, "1.24.5");
}

#[test]
fn test_install_plan_missing_info_skipped() {
    let mut graph = DependencyGraph::new();
    graph.add_node("wget");
    graph.add_node("unknown");

    let plan = InstallPlan::from_graph(
        &graph,
        &["wget", "unknown"],
        |name| {
            if name == "wget" {
                Some(make_formula_info(name, "1.0.0"))
            } else {
                None // unknown has no info
            }
        },
        |_| false,
    )
    .unwrap();

    // Only wget should be in steps
    assert_eq!(plan.total_packages(), 1);
    assert_eq!(plan.steps[0].name, "wget");
}

#[test]
fn test_install_plan_new_packages_iterator() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("wget", "openssl");

    let plan = InstallPlan::from_graph(
        &graph,
        &["wget"],
        |name| Some(make_formula_info(name, "1.0.0")),
        |_| false,
    )
    .unwrap();

    let new_pkgs: Vec<_> = plan.new_packages().collect();
    assert_eq!(new_pkgs.len(), 2);
}

// ============================================================================
// Error type tests
// ============================================================================

#[test]
fn test_error_display_cycle() {
    let err = Error::CycleDetected("a, b, c".to_string());
    assert_eq!(err.to_string(), "Dependency cycle detected: a, b, c");
}

#[test]
fn test_error_display_unresolved() {
    let err = Error::UnresolvedDependency("wget".to_string(), "openssl".to_string());
    assert_eq!(
        err.to_string(),
        "Unresolved dependency: wget requires openssl"
    );
}

#[test]
fn test_error_display_conflict() {
    let err = Error::Conflict("openssl@3".to_string(), "openssl@1.1".to_string());
    assert_eq!(
        err.to_string(),
        "Conflict: openssl@3 conflicts with openssl@1.1"
    );
}
