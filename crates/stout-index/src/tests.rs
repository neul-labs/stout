//! Tests for stout-index

use crate::db::Database;
use crate::formula::{DependencyType, FormulaInfo};
use crate::query::Query;
use tempfile::tempdir;

#[test]
fn test_database_creation() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let db = Database::open(&db_path).unwrap();

    assert!(!db.is_initialized().unwrap());
    assert_eq!(db.formula_count().unwrap(), 0);
}

#[test]
fn test_database_formula_operations() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut db = Database::open(&db_path).unwrap();

    // Insert a formula
    {
        let tx = db.transaction().unwrap();
        tx.upsert_formula(&FormulaInfo {
            name: "wget".to_string(),
            version: "1.24.5".to_string(),
            revision: 0,
            desc: Some("Internet file retriever".to_string()),
            homepage: Some("https://www.gnu.org/software/wget/".to_string()),
            license: Some("GPL-3.0-or-later".to_string()),
            tap: "homebrew/core".to_string(),
            deprecated: false,
            disabled: false,
            has_bottle: true,
            json_hash: Some("abc123".to_string()),
        })
        .unwrap();

        tx.insert_dependency("wget", "openssl@3", DependencyType::Runtime)
            .unwrap();
        tx.insert_dependency("wget", "pkg-config", DependencyType::Build)
            .unwrap();
        tx.insert_bottle("wget", "arm64_sonoma").unwrap();
        tx.insert_bottle("wget", "x86_64_linux").unwrap();
        tx.set_meta("version", "2024.01.01").unwrap();
        tx.commit().unwrap();
    }

    // Verify formula was inserted
    let formula = db.get_formula("wget").unwrap().unwrap();
    assert_eq!(formula.name, "wget");
    assert_eq!(formula.version, "1.24.5");
    assert_eq!(formula.desc, Some("Internet file retriever".to_string()));

    // Verify version is set
    assert_eq!(db.version().unwrap(), Some("2024.01.01".to_string()));
    assert!(db.is_initialized().unwrap());

    // Verify dependencies
    let deps = db.get_dependencies("wget").unwrap();
    assert_eq!(deps.len(), 2);
    assert!(deps
        .iter()
        .any(|d| d.name == "openssl@3" && d.dep_type == DependencyType::Runtime));
    assert!(deps
        .iter()
        .any(|d| d.name == "pkg-config" && d.dep_type == DependencyType::Build));

    // Verify bottles
    let platforms = db.get_platforms("wget").unwrap();
    assert!(platforms.contains(&"arm64_sonoma".to_string()));
    assert!(platforms.contains(&"x86_64_linux".to_string()));
}

#[test]
fn test_database_search() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut db = Database::open(&db_path).unwrap();

    // Insert test formulas
    {
        let tx = db.transaction().unwrap();

        for (name, desc) in [
            ("jq", "Command-line JSON processor"),
            ("jless", "Command-line JSON viewer"),
            ("gojq", "Pure Go implementation of jq"),
            ("wget", "Internet file retriever"),
        ] {
            tx.upsert_formula(&FormulaInfo {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                revision: 0,
                desc: Some(desc.to_string()),
                homepage: None,
                license: None,
                tap: "homebrew/core".to_string(),
                deprecated: false,
                disabled: false,
                has_bottle: true,
                json_hash: None,
            })
            .unwrap();
        }

        tx.set_meta("version", "test").unwrap();
        tx.commit().unwrap();
    }

    // Search for JSON-related
    let results = db.search("json*", 10).unwrap();
    assert!(results.len() >= 1);
    assert!(results
        .iter()
        .any(|f| f.name == "jq" || f.name == "jless" || f.name == "gojq"));

    // Search for specific name
    let results = db.search("wget", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "wget");
}

#[test]
fn test_database_find_similar() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut db = Database::open(&db_path).unwrap();

    {
        let tx = db.transaction().unwrap();

        for name in ["node", "neotest", "nextest", "wget", "curl"] {
            tx.upsert_formula(&FormulaInfo {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                revision: 0,
                desc: None,
                homepage: None,
                license: None,
                tap: "homebrew/core".to_string(),
                deprecated: false,
                disabled: false,
                has_bottle: true,
                json_hash: None,
            })
            .unwrap();
        }

        tx.commit().unwrap();
    }

    // Find similar to "nod" (should suggest "node")
    let suggestions = db.find_similar("nod", 5).unwrap();
    assert!(suggestions.contains(&"node".to_string()));

    // Find similar to "neo" (should suggest "neotest")
    let suggestions = db.find_similar("neo", 5).unwrap();
    assert!(suggestions.contains(&"neotest".to_string()));
}

#[test]
fn test_database_list_formulas() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut db = Database::open(&db_path).unwrap();

    {
        let tx = db.transaction().unwrap();

        for name in ["aaa", "bbb", "ccc", "ddd", "eee"] {
            tx.upsert_formula(&FormulaInfo {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                revision: 0,
                desc: None,
                homepage: None,
                license: None,
                tap: "homebrew/core".to_string(),
                deprecated: false,
                disabled: false,
                has_bottle: true,
                json_hash: None,
            })
            .unwrap();
        }

        tx.commit().unwrap();
    }

    // List all
    let all = db.list_formulas(0, 100).unwrap();
    assert_eq!(all.len(), 5);

    // List with pagination
    let page1 = db.list_formulas(0, 2).unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].name, "aaa");
    assert_eq!(page1[1].name, "bbb");

    let page2 = db.list_formulas(2, 2).unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].name, "ccc");
    assert_eq!(page2[1].name, "ddd");
}

#[test]
fn test_query_interface() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut db = Database::open(&db_path).unwrap();

    {
        let tx = db.transaction().unwrap();
        tx.upsert_formula(&FormulaInfo {
            name: "wget".to_string(),
            version: "1.24.5".to_string(),
            revision: 0,
            desc: Some("Internet file retriever".to_string()),
            homepage: None,
            license: None,
            tap: "homebrew/core".to_string(),
            deprecated: false,
            disabled: false,
            has_bottle: true,
            json_hash: None,
        })
        .unwrap();
        tx.set_meta("version", "test").unwrap();
        tx.commit().unwrap();
    }

    let query = Query::new(&db);

    // Test get
    let formula = query.get("wget").unwrap();
    assert_eq!(formula.name, "wget");

    // Test get_opt
    let found = query.get_opt("wget").unwrap();
    assert!(found.is_some());

    let not_found = query.get_opt("nonexistent").unwrap();
    assert!(not_found.is_none());

    // Test exists
    assert!(query.exists("wget").unwrap());
    assert!(!query.exists("nonexistent").unwrap());

    // Test count
    assert_eq!(query.count().unwrap(), 1);
}

#[test]
fn test_formula_info_serialization() {
    let info = FormulaInfo {
        name: "wget".to_string(),
        version: "1.24.5".to_string(),
        revision: 1,
        desc: Some("Test description".to_string()),
        homepage: Some("https://example.com".to_string()),
        license: Some("MIT".to_string()),
        tap: "homebrew/core".to_string(),
        deprecated: false,
        disabled: false,
        has_bottle: true,
        json_hash: Some("abc123".to_string()),
    };

    let json = serde_json::to_string(&info).unwrap();
    let parsed: FormulaInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, info.name);
    assert_eq!(parsed.version, info.version);
    assert_eq!(parsed.revision, info.revision);
}

#[test]
fn test_get_dependents() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut db = Database::open(&db_path).unwrap();

    // Insert formulas and dependencies: curl -> openssl@3, wget -> openssl@3 (runtime), wget -> pkg-config (build)
    {
        let tx = db.transaction().unwrap();
        for name in ["openssl@3", "wget", "curl", "pkg-config"] {
            tx.upsert_formula(&FormulaInfo {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                revision: 0,
                desc: None,
                homepage: None,
                license: None,
                tap: "homebrew/core".to_string(),
                deprecated: false,
                disabled: false,
                has_bottle: true,
                json_hash: None,
            })
            .unwrap();
        }
        tx.insert_dependency("wget", "openssl@3", DependencyType::Runtime)
            .unwrap();
        tx.insert_dependency("wget", "pkg-config", DependencyType::Build)
            .unwrap();
        tx.insert_dependency("curl", "openssl@3", DependencyType::Runtime)
            .unwrap();
        tx.insert_dependency("curl", "pkg-config", DependencyType::Optional)
            .unwrap();
        tx.commit().unwrap();
    }

    // Test: who depends on openssl@3? (wget runtime, curl runtime)
    let dependents = db.get_dependents("openssl@3", &[]).unwrap();
    assert_eq!(dependents.len(), 2);
    assert!(dependents
        .iter()
        .any(|d| d.formula == "wget" && d.dep_type == DependencyType::Runtime));
    assert!(dependents
        .iter()
        .any(|d| d.formula == "curl" && d.dep_type == DependencyType::Runtime));

    // Test: filter by dep_type = Runtime only
    let runtime_only = db
        .get_dependents("openssl@3", &[DependencyType::Runtime])
        .unwrap();
    assert_eq!(runtime_only.len(), 2);

    // Test: filter by dep_type = Build (no build deps on openssl@3)
    let build_only = db
        .get_dependents("openssl@3", &[DependencyType::Build])
        .unwrap();
    assert_eq!(build_only.len(), 0);

    // Test: who depends on pkg-config? (wget build, curl optional)
    let pkg_config_deps = db.get_dependents("pkg-config", &[]).unwrap();
    assert_eq!(pkg_config_deps.len(), 2);

    // Test: filter pkg-config dependents by Build type only
    let pkg_config_build = db
        .get_dependents("pkg-config", &[DependencyType::Build])
        .unwrap();
    assert_eq!(pkg_config_build.len(), 1);
    assert_eq!(pkg_config_build[0].formula, "wget");

    // Test: no dependents
    let no_deps = db.get_dependents("nonexistent", &[]).unwrap();
    assert!(no_deps.is_empty());
}
