use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::types::PolicyRecord;

/// In-memory cache of policy records, shared across all evaluators.
///
/// Uses `Arc<RwLock<…>>` so cloning `PolicyCache` shares the underlying map. The
/// lock is held only for brief HashMap operations — never across an await point.
///
/// **Scaffold note:** the production build also *loads and refreshes* these
/// records from Postgres. That persistence layer is intentionally omitted from
/// this standalone crate — it is service-coupled (connection pool + error type)
/// and is reattached when the engine is wired into the identity service. The
/// in-memory store plus the evaluator logic is the reviewable core.
#[derive(Clone, Default)]
pub struct PolicyCache {
    inner: Arc<RwLock<HashMap<String, PolicyRecord>>>,
}

impl PolicyCache {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<PolicyRecord> {
        // Recover from a poisoned lock (a panic while writing) rather than
        // failing: the data inside is still valid; only the poison flag needs
        // clearing.
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    /// Insert or replace a single record (used after a policy is edited).
    pub fn upsert(&self, record: PolicyRecord) {
        let mut map = self.inner.write().unwrap_or_else(|e| e.into_inner());
        map.insert(record.key.clone(), record);
    }

    pub fn all(&self) -> Vec<PolicyRecord> {
        let map = self.inner.read().unwrap_or_else(|e| e.into_inner());
        let mut records: Vec<PolicyRecord> = map.values().cloned().collect();
        records.sort_by(|a, b| a.key.cmp(&b.key));
        records
    }
}
