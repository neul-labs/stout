//! Query interface for the index

use crate::db::Database;
use crate::error::{Error, Result};
use crate::formula::FormulaInfo;

/// High-level query interface
pub struct Query<'a> {
    db: &'a Database,
}

impl<'a> Query<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get a formula by exact name
    pub fn get(&self, name: &str) -> Result<FormulaInfo> {
        self.db
            .get_formula(name)?
            .ok_or_else(|| Error::FormulaNotFound(name.to_string()))
    }

    /// Get a formula, returning None if not found
    pub fn get_opt(&self, name: &str) -> Result<Option<FormulaInfo>> {
        self.db.get_formula(name)
    }

    /// Search formulas by query string
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<FormulaInfo>> {
        // Prepare query for FTS5
        let fts_query = prepare_fts_query(query);
        self.db.search(&fts_query, limit)
    }

    /// List all formulas
    pub fn list(&self, offset: usize, limit: usize) -> Result<Vec<FormulaInfo>> {
        self.db.list_formulas(offset, limit)
    }

    /// Get suggestions for a misspelled name
    pub fn suggest(&self, name: &str, limit: usize) -> Result<Vec<String>> {
        self.db.find_similar(name, limit)
    }

    /// Check if a formula exists
    pub fn exists(&self, name: &str) -> Result<bool> {
        Ok(self.db.get_formula(name)?.is_some())
    }

    /// Get formula count
    pub fn count(&self) -> Result<u32> {
        self.db.formula_count()
    }
}

/// Prepare a user query for FTS5
fn prepare_fts_query(query: &str) -> String {
    // Simple tokenization - in production might want more sophisticated handling
    let tokens: Vec<&str> = query.split_whitespace().collect();

    if tokens.len() == 1 {
        // Single word: prefix match
        format!("{}*", tokens[0])
    } else {
        // Multiple words: AND them together with prefix on last
        let mut parts: Vec<String> = tokens[..tokens.len() - 1]
            .iter()
            .map(|t| t.to_string())
            .collect();
        parts.push(format!("{}*", tokens.last().unwrap()));
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_fts_query() {
        assert_eq!(prepare_fts_query("json"), "json*");
        assert_eq!(prepare_fts_query("json parser"), "json parser*");
        assert_eq!(prepare_fts_query("command line tool"), "command line tool*");
    }
}
