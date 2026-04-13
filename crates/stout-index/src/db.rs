//! Database operations

use crate::cask::CaskInfo;
use crate::error::Result;
use crate::formula::{Dependency, DependencyType, FormulaInfo};
use crate::schema::{meta_keys, CREATE_SCHEMA};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::debug;

/// SQLite database for formula index
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    /// Open or create a database at the given path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        debug!("Opening database at {}", path.display());

        let conn = Connection::open(&path)?;

        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        let db = Self { conn, path };
        db.init_schema()?;

        Ok(db)
    }

    /// Open an in-memory database (for testing)
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(CREATE_SCHEMA)?;
        Ok(())
    }

    /// Get the database path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get a metadata value
    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let value = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?", [key], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(value)
    }

    /// Set a metadata value
    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?, ?)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get the index version
    pub fn version(&self) -> Result<Option<String>> {
        self.get_meta(meta_keys::VERSION)
    }

    /// Get the formula count
    pub fn formula_count(&self) -> Result<u32> {
        let count: u32 = self
            .conn
            .query_row("SELECT COUNT(*) FROM formulas", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Check if the database is initialized
    pub fn is_initialized(&self) -> Result<bool> {
        Ok(self.version()?.is_some())
    }

    /// Look up a formula by name
    pub fn get_formula(&self, name: &str) -> Result<Option<FormulaInfo>> {
        let formula = self
            .conn
            .query_row(
                r#"
                SELECT name, version, revision, desc, homepage, license, tap,
                       deprecated, disabled, has_bottle, json_hash
                FROM formulas
                WHERE name = ?
                "#,
                [name],
                |row| {
                    Ok(FormulaInfo {
                        name: row.get(0)?,
                        version: row.get(1)?,
                        revision: row.get(2)?,
                        desc: row.get(3)?,
                        homepage: row.get(4)?,
                        license: row.get(5)?,
                        tap: row.get(6)?,
                        deprecated: row.get(7)?,
                        disabled: row.get(8)?,
                        has_bottle: row.get(9)?,
                        json_hash: row.get(10)?,
                    })
                },
            )
            .optional()?;
        Ok(formula)
    }

    /// Search formulas using FTS
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<FormulaInfo>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT f.name, f.version, f.revision, f.desc, f.homepage, f.license, f.tap,
                   f.deprecated, f.disabled, f.has_bottle, f.json_hash
            FROM formulas f
            JOIN formulas_fts fts ON f.rowid = fts.rowid
            WHERE formulas_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )?;

        let formulas = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(FormulaInfo {
                    name: row.get(0)?,
                    version: row.get(1)?,
                    revision: row.get(2)?,
                    desc: row.get(3)?,
                    homepage: row.get(4)?,
                    license: row.get(5)?,
                    tap: row.get(6)?,
                    deprecated: row.get(7)?,
                    disabled: row.get(8)?,
                    has_bottle: row.get(9)?,
                    json_hash: row.get(10)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(formulas)
    }

    /// List all formulas (paginated)
    pub fn list_formulas(&self, offset: usize, limit: usize) -> Result<Vec<FormulaInfo>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT name, version, revision, desc, homepage, license, tap,
                   deprecated, disabled, has_bottle, json_hash
            FROM formulas
            ORDER BY name
            LIMIT ? OFFSET ?
            "#,
        )?;

        let formulas = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Ok(FormulaInfo {
                    name: row.get(0)?,
                    version: row.get(1)?,
                    revision: row.get(2)?,
                    desc: row.get(3)?,
                    homepage: row.get(4)?,
                    license: row.get(5)?,
                    tap: row.get(6)?,
                    deprecated: row.get(7)?,
                    disabled: row.get(8)?,
                    has_bottle: row.get(9)?,
                    json_hash: row.get(10)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(formulas)
    }

    /// Get dependencies for a formula
    pub fn get_dependencies(&self, formula: &str) -> Result<Vec<Dependency>> {
        let mut stmt = self
            .conn
            .prepare("SELECT dep_name, dep_type FROM dependencies WHERE formula = ?")?;

        let deps = stmt
            .query_map([formula], |row| {
                let name: String = row.get(0)?;
                let dep_type_str: String = row.get(1)?;
                let dep_type = match dep_type_str.as_str() {
                    "runtime" => DependencyType::Runtime,
                    "build" => DependencyType::Build,
                    "test" => DependencyType::Test,
                    "optional" => DependencyType::Optional,
                    "recommended" => DependencyType::Recommended,
                    _ => DependencyType::Runtime,
                };
                Ok(Dependency { name, dep_type })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(deps)
    }

    /// Get available platforms for a formula
    pub fn get_platforms(&self, formula: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT platform FROM bottles WHERE formula = ?")?;

        let platforms = stmt
            .query_map([formula], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(platforms)
    }

    /// Find similar formula names (for "did you mean?" suggestions)
    pub fn find_similar(&self, name: &str, limit: usize) -> Result<Vec<String>> {
        // Simple prefix/contains search for suggestions
        let mut stmt = self.conn.prepare(
            r#"
            SELECT name FROM formulas
            WHERE name LIKE ? OR name LIKE ?
            ORDER BY
                CASE
                    WHEN name LIKE ? THEN 0
                    ELSE 1
                END,
                length(name)
            LIMIT ?
            "#,
        )?;

        let pattern_prefix = format!("{}%", name);
        let pattern_contains = format!("%{}%", name);

        let names = stmt
            .query_map(
                params![
                    pattern_prefix,
                    pattern_contains,
                    pattern_prefix,
                    limit as i64
                ],
                |row| row.get(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(names)
    }

    // ==================== Cask methods ====================

    /// Get the cask count
    pub fn cask_count(&self) -> Result<u32> {
        let count: u32 = self
            .conn
            .query_row("SELECT COUNT(*) FROM casks", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Look up a cask by token
    pub fn get_cask(&self, token: &str) -> Result<Option<CaskInfo>> {
        let cask = self
            .conn
            .query_row(
                r#"
                SELECT token, name, version, desc, homepage, tap,
                       deprecated, disabled, artifact_type, json_hash
                FROM casks
                WHERE token = ?
                "#,
                [token],
                |row| {
                    Ok(CaskInfo {
                        token: row.get(0)?,
                        name: row.get(1)?,
                        version: row.get(2)?,
                        desc: row.get(3)?,
                        homepage: row.get(4)?,
                        tap: row.get(5)?,
                        deprecated: row.get(6)?,
                        disabled: row.get(7)?,
                        artifact_type: row.get(8)?,
                        json_hash: row.get(9)?,
                    })
                },
            )
            .optional()?;
        Ok(cask)
    }

    /// Search casks using FTS
    pub fn search_casks(&self, query: &str, limit: usize) -> Result<Vec<CaskInfo>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT c.token, c.name, c.version, c.desc, c.homepage, c.tap,
                   c.deprecated, c.disabled, c.artifact_type, c.json_hash
            FROM casks c
            JOIN casks_fts fts ON c.rowid = fts.rowid
            WHERE casks_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )?;

        let casks = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok(CaskInfo {
                    token: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    desc: row.get(3)?,
                    homepage: row.get(4)?,
                    tap: row.get(5)?,
                    deprecated: row.get(6)?,
                    disabled: row.get(7)?,
                    artifact_type: row.get(8)?,
                    json_hash: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(casks)
    }

    /// List all casks (paginated)
    pub fn list_casks(&self, offset: usize, limit: usize) -> Result<Vec<CaskInfo>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT token, name, version, desc, homepage, tap,
                   deprecated, disabled, artifact_type, json_hash
            FROM casks
            ORDER BY token
            LIMIT ? OFFSET ?
            "#,
        )?;

        let casks = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Ok(CaskInfo {
                    token: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    desc: row.get(3)?,
                    homepage: row.get(4)?,
                    tap: row.get(5)?,
                    deprecated: row.get(6)?,
                    disabled: row.get(7)?,
                    artifact_type: row.get(8)?,
                    json_hash: row.get(9)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(casks)
    }

    /// Find similar cask tokens (for "did you mean?" suggestions)
    pub fn find_similar_casks(&self, token: &str, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT token FROM casks
            WHERE token LIKE ? OR token LIKE ? OR name LIKE ?
            ORDER BY
                CASE
                    WHEN token LIKE ? THEN 0
                    ELSE 1
                END,
                length(token)
            LIMIT ?
            "#,
        )?;

        let pattern_prefix = format!("{}%", token);
        let pattern_contains = format!("%{}%", token);

        let tokens = stmt
            .query_map(
                params![
                    pattern_prefix,
                    pattern_contains,
                    pattern_contains,
                    pattern_prefix,
                    limit as i64
                ],
                |row| row.get(0),
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tokens)
    }

    /// Search both formulas and casks
    pub fn search_all(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<(Vec<FormulaInfo>, Vec<CaskInfo>)> {
        let formulas = self.search(query, limit)?;
        let casks = self.search_casks(query, limit)?;
        Ok((formulas, casks))
    }

    /// Begin a transaction for bulk operations
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        Ok(Transaction {
            tx: self.conn.transaction()?,
        })
    }

    /// Import casks from another database, replacing all existing casks
    ///
    /// This handles schema differences between the source database and the local schema.
    pub fn import_casks_from(&mut self, source: &Database) -> Result<u32> {
        // Clear existing casks and dependencies
        self.conn.execute("DELETE FROM cask_dependencies", [])?;
        self.conn.execute("DELETE FROM casks", [])?;

        // Query the source database schema to detect available columns
        let source_columns: HashSet<String> = source
            .conn
            .prepare("PRAGMA table_info(casks)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<std::result::Result<_, _>>()?;

        let mut count = 0u32;
        const BATCH_SIZE: usize = 1000;

        // Build a query that works with the source schema
        // The source may have different columns than our local schema
        let has_artifact_type = source_columns.contains("artifact_type");

        let query = if has_artifact_type {
            r#"
            SELECT token, name, version, desc, homepage, tap,
                   deprecated, disabled, artifact_type, json_hash
            FROM casks
            ORDER BY token
            LIMIT ? OFFSET ?
            "#
        } else {
            r#"
            SELECT token, name, version, desc, homepage, tap,
                   deprecated, disabled, NULL as artifact_type, json_hash
            FROM casks
            ORDER BY token
            LIMIT ? OFFSET ?
            "#
        };

        let mut offset = 0;

        loop {
            let mut stmt = source.conn.prepare(query)?;
            let casks: Vec<CaskInfo> = stmt
                .query_map(params![BATCH_SIZE as i64, offset as i64], |row| {
                    Ok(CaskInfo {
                        token: row.get(0)?,
                        name: row.get(1)?,
                        version: row.get(2)?,
                        desc: row.get(3)?,
                        homepage: row.get(4)?,
                        tap: row.get(5)?,
                        deprecated: row.get(6)?,
                        disabled: row.get(7)?,
                        artifact_type: row.get(8)?,
                        json_hash: row.get(9)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            if casks.is_empty() {
                break;
            }

            let tx = self.conn.transaction()?;
            for cask in &casks {
                tx.execute(
                    r#"
                    INSERT INTO casks
                    (token, name, version, desc, homepage, tap, deprecated, disabled, artifact_type, json_hash)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    params![
                        cask.token,
                        cask.name,
                        cask.version,
                        cask.desc,
                        cask.homepage,
                        cask.tap,
                        cask.deprecated,
                        cask.disabled,
                        cask.artifact_type,
                        cask.json_hash,
                    ],
                )?;
                count += 1;
            }
            tx.commit()?;

            offset += BATCH_SIZE;
            if casks.len() < BATCH_SIZE {
                break;
            }
        }

        Ok(count)
    }
}

/// A database transaction for bulk operations
pub struct Transaction<'a> {
    tx: rusqlite::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    /// Insert or update a formula
    pub fn upsert_formula(&self, formula: &FormulaInfo) -> Result<()> {
        self.tx.execute(
            r#"
            INSERT OR REPLACE INTO formulas
            (name, version, revision, desc, homepage, license, tap, deprecated, disabled, has_bottle, json_hash)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                formula.name,
                formula.version,
                formula.revision,
                formula.desc,
                formula.homepage,
                formula.license,
                formula.tap,
                formula.deprecated,
                formula.disabled,
                formula.has_bottle,
                formula.json_hash,
            ],
        )?;
        Ok(())
    }

    /// Insert a dependency
    pub fn insert_dependency(
        &self,
        formula: &str,
        dep_name: &str,
        dep_type: DependencyType,
    ) -> Result<()> {
        self.tx.execute(
            "INSERT OR IGNORE INTO dependencies (formula, dep_name, dep_type) VALUES (?, ?, ?)",
            params![formula, dep_name, dep_type.as_str()],
        )?;
        Ok(())
    }

    /// Insert a bottle platform
    pub fn insert_bottle(&self, formula: &str, platform: &str) -> Result<()> {
        self.tx.execute(
            "INSERT OR IGNORE INTO bottles (formula, platform) VALUES (?, ?)",
            params![formula, platform],
        )?;
        Ok(())
    }

    /// Insert an alias
    pub fn insert_alias(&self, alias: &str, formula: &str) -> Result<()> {
        self.tx.execute(
            "INSERT OR IGNORE INTO aliases (alias, formula) VALUES (?, ?)",
            params![alias, formula],
        )?;
        Ok(())
    }

    /// Set metadata
    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.tx.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?, ?)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Clear all formula data (for full rebuild)
    pub fn clear_all(&self) -> Result<()> {
        self.tx.execute_batch(
            r#"
            DELETE FROM dependencies;
            DELETE FROM bottles;
            DELETE FROM aliases;
            DELETE FROM formulas;
            DELETE FROM cask_dependencies;
            DELETE FROM casks;
            "#,
        )?;
        Ok(())
    }

    // ==================== Cask transaction methods ====================

    /// Insert or update a cask
    pub fn upsert_cask(&self, cask: &CaskInfo) -> Result<()> {
        self.tx.execute(
            r#"
            INSERT OR REPLACE INTO casks
            (token, name, version, desc, homepage, tap, deprecated, disabled, artifact_type, json_hash)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                cask.token,
                cask.name,
                cask.version,
                cask.desc,
                cask.homepage,
                cask.tap,
                cask.deprecated,
                cask.disabled,
                cask.artifact_type,
                cask.json_hash,
            ],
        )?;
        Ok(())
    }

    /// Insert a cask dependency
    pub fn insert_cask_dependency(&self, cask: &str, dep_name: &str, dep_type: &str) -> Result<()> {
        self.tx.execute(
            "INSERT OR IGNORE INTO cask_dependencies (cask, dep_name, dep_type) VALUES (?, ?, ?)",
            params![cask, dep_name, dep_type],
        )?;
        Ok(())
    }

    /// Commit the transaction
    pub fn commit(self) -> Result<()> {
        self.tx.commit()?;
        Ok(())
    }
}
