//! Database schema definitions

/// SQL to create the index database schema
pub const CREATE_SCHEMA: &str = r#"
-- Formula metadata (fast queries, search, listing)
CREATE TABLE IF NOT EXISTS formulas (
    name TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    revision INTEGER DEFAULT 0,
    desc TEXT,
    homepage TEXT,
    license TEXT,
    tap TEXT DEFAULT 'homebrew/core',
    deprecated INTEGER DEFAULT 0,
    disabled INTEGER DEFAULT 0,
    has_bottle INTEGER DEFAULT 1,
    json_hash TEXT,
    updated_at INTEGER
);

-- Full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS formulas_fts USING fts5(
    name, desc,
    content='formulas',
    content_rowid='rowid'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS formulas_ai AFTER INSERT ON formulas BEGIN
    INSERT INTO formulas_fts(rowid, name, desc) VALUES (NEW.rowid, NEW.name, NEW.desc);
END;

CREATE TRIGGER IF NOT EXISTS formulas_ad AFTER DELETE ON formulas BEGIN
    INSERT INTO formulas_fts(formulas_fts, rowid, name, desc) VALUES('delete', OLD.rowid, OLD.name, OLD.desc);
END;

CREATE TRIGGER IF NOT EXISTS formulas_au AFTER UPDATE ON formulas BEGIN
    INSERT INTO formulas_fts(formulas_fts, rowid, name, desc) VALUES('delete', OLD.rowid, OLD.name, OLD.desc);
    INSERT INTO formulas_fts(rowid, name, desc) VALUES (NEW.rowid, NEW.name, NEW.desc);
END;

-- Dependencies (for quick dependency queries without fetching JSON)
CREATE TABLE IF NOT EXISTS dependencies (
    formula TEXT NOT NULL,
    dep_name TEXT NOT NULL,
    dep_type TEXT NOT NULL,
    PRIMARY KEY (formula, dep_name, dep_type),
    FOREIGN KEY (formula) REFERENCES formulas(name)
);

-- Bottle availability matrix (quick platform compatibility check)
CREATE TABLE IF NOT EXISTS bottles (
    formula TEXT NOT NULL,
    platform TEXT NOT NULL,
    PRIMARY KEY (formula, platform),
    FOREIGN KEY (formula) REFERENCES formulas(name)
);

-- Aliases and old names (for search)
CREATE TABLE IF NOT EXISTS aliases (
    alias TEXT PRIMARY KEY,
    formula TEXT NOT NULL,
    FOREIGN KEY (formula) REFERENCES formulas(name)
);

-- Index metadata
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT
);

-- Indexes for faster queries
CREATE INDEX IF NOT EXISTS idx_dependencies_formula ON dependencies(formula);
CREATE INDEX IF NOT EXISTS idx_dependencies_dep ON dependencies(dep_name);
CREATE INDEX IF NOT EXISTS idx_bottles_formula ON bottles(formula);
CREATE INDEX IF NOT EXISTS idx_formulas_tap ON formulas(tap);
"#;

/// Meta keys
pub mod meta_keys {
    pub const VERSION: &str = "version";
    pub const CREATED_AT: &str = "created_at";
    pub const HOMEBREW_COMMIT: &str = "homebrew_commit";
    pub const FORMULA_COUNT: &str = "formula_count";
}
