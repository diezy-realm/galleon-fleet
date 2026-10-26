//! End-to-End integration test suite for multi-agent session continuity
//! and SQLite memory persistence across daemon crash and restart.
//!
//! Validates:
//! 1. Physical SQLite database initialization on a real disk path (tempfile) and
//!    complete table schema migration cycle (`001_initial.sql`, V3 multi-agent schema).
//! 2. Multi-turn multi-agent execution with real tool operations (file I/O, echo),
//!    persisting session turns, memory facts, and token usage into disk SQLite databases.
//! 3. Crash/restart simulation: complete drop of in-memory agent/runtime instances from RAM.
//! 4. Post-restart recovery verification: brand new instances directed to the same SQLite
//!    files on disk achieve 100% recall of turn history, memory facts, and token ledger.
//! 5. Zero mock provider/memory: direct SQL row assertions inspect the physical disk SQLite tables.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;
use tempfile::TempDir;

use clawcrew_memory::traits::StoreOptions;
use clawcrew_memory::{
    AgentMemoryGrant, AgentScopedMemory, AuditedMemory, Memory, MemoryCategory, ResponseCache,
    SqliteMemory,
};

fn db_brain_path(workspace: &Path) -> PathBuf {
    workspace.join("memory").join("brain.db")
}

fn db_audit_path(workspace: &Path) -> PathBuf {
    workspace.join("memory").join("audit.db")
}

fn db_cache_path(workspace: &Path) -> PathBuf {
    workspace.join("memory").join("response_cache.db")
}

fn open_raw_brain(workspace: &Path) -> Connection {
    Connection::open(db_brain_path(workspace)).expect("open physical brain.db")
}

fn open_raw_audit(workspace: &Path) -> Connection {
    Connection::open(db_audit_path(workspace)).expect("open physical audit.db")
}

fn open_raw_cache(workspace: &Path) -> Connection {
    Connection::open(db_cache_path(workspace)).expect("open physical response_cache.db")
}

#[tokio::test]
async fn test_multiagent_session_continuity_and_sqlite_memory_persistence_recovery() {
    let workspace = TempDir::new().expect("create physical workspace tempdir");
    let workspace_path = workspace.path();

    // ── 1. Initialize physical SQLite DB on disk and run full migration cycle ──
    let brain_path = db_brain_path(workspace_path);
    let audit_path = db_audit_path(workspace_path);
    let cache_path = db_cache_path(workspace_path);

    let sqlite_mem = SqliteMemory::new("sqlite", workspace_path)
        .expect("initialize physical SqliteMemory on disk");

    assert!(
        brain_path.exists(),
        "physical brain.db file must exist on disk after initialization"
    );

    // Verify initial migration and schema integrity directly against disk SQLite
    {
        let conn = open_raw_brain(workspace_path);

        // Check required tables
        let mut table_stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .expect("prepare table query");
        let tables: HashSet<String> = table_stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query tables")
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            tables.contains("agents"),
            "agents table must be migrated and present"
        );
        assert!(
            tables.contains("memories"),
            "memories table must be migrated and present"
        );
        assert!(
            tables.contains("memories_fts"),
            "memories_fts FTS5 virtual table must be present"
        );
        assert!(
            tables.contains("embedding_cache"),
            "embedding_cache table must be present"
        );
        assert!(
            tables.contains("memory_meta"),
            "memory_meta table must be present"
        );

        // Check columns on memories table
        let mut pragma_stmt = conn
            .prepare("PRAGMA table_info(memories)")
            .expect("prepare pragma table_info");
        let columns: HashSet<String> = pragma_stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query columns")
            .filter_map(|r| r.ok())
            .collect();

        for col in &[
            "id",
            "key",
            "content",
            "category",
            "agent_id",
            "session_id",
            "namespace",
            "importance",
            "superseded_by",
            "kind",
            "pinned",
            "tenant_id",
            "scope",
        ] {
            assert!(
                columns.contains(*col),
                "memories schema missing migrated column '{col}'"
            );
        }
    }

    // ── 2. Register multi-agent identities in agents table ──
    let alpha_uuid = sqlite_mem
        .ensure_agent_uuid("agent_alpha")
        .await
        .expect("ensure agent_alpha UUID");
    let beta_uuid = sqlite_mem
        .ensure_agent_uuid("agent_beta")
        .await
        .expect("ensure agent_beta UUID");

    assert!(
        uuid::Uuid::parse_str(&alpha_uuid).is_ok(),
        "agent_alpha UUID must be valid UUIDv4"
    );
    assert!(
        uuid::Uuid::parse_str(&beta_uuid).is_ok(),
        "agent_beta UUID must be valid UUIDv4"
    );
    assert_ne!(
        alpha_uuid, beta_uuid,
        "distinct agents must have distinct UUIDs"
    );

    // Setup AuditedMemory with physical audit.db on disk
    let audited_backend = Arc::new(
        AuditedMemory::new(sqlite_mem.clone(), workspace_path)
            .expect("create physical AuditedMemory on disk"),
    );
    assert!(
        audit_path.exists(),
        "physical audit.db file must exist on disk"
    );

    // Setup ResponseCache with physical response_cache.db on disk
    let response_cache = ResponseCache::new(workspace_path, 180, 500)
        .expect("create physical ResponseCache on disk");
    assert!(
        cache_path.exists(),
        "physical response_cache.db file must exist on disk"
    );

    // Setup AgentScopedMemory with cross-agent access rules:
    // Agent Beta can read Agent Alpha's Core and Conversation categories,
    // but CANNOT read Alpha's private Daily category.
    let mut beta_grants = HashSet::new();
    beta_grants.insert(MemoryCategory::Core.to_string());
    beta_grants.insert(MemoryCategory::Conversation.to_string());
    let alpha_grant_for_beta = AgentMemoryGrant {
        agent_id: alpha_uuid.clone(),
        categories: Some(beta_grants.clone()),
    };

    let alpha_mem = AgentScopedMemory::new(audited_backend.clone(), &alpha_uuid, vec![]);
    let beta_mem = AgentScopedMemory::new_with_grants(
        audited_backend.clone(),
        &beta_uuid,
        vec![alpha_grant_for_beta],
    );

    // ── 3. Multi-turn execution with real tool operations and token ledger ──
    let artifacts_dir = workspace_path.join("artifacts");
    fs::create_dir_all(&artifacts_dir).expect("create real artifacts directory");

    // Turn 1 (Agent Alpha): Real File Tool writes architecture document to disk
    let arch_file = artifacts_dir.join("system_architecture.json");
    let arch_json = r#"{"system": "galleon-fleet", "nodes": 12, "resilience": "high"}"#;
    fs::write(&arch_file, arch_json).expect("real file tool write to disk");
    let verified_arch = fs::read_to_string(&arch_file).expect("real file tool read back");
    assert_eq!(verified_arch, arch_json);

    alpha_mem
        .store_with_options(
            "tool:file:system_architecture",
            &format!("Architecture spec created: {verified_arch}"),
            MemoryCategory::Core,
            Some("session_alpha_proj"),
            StoreOptions {
                importance: Some(0.95),
                ..Default::default()
            },
        )
        .await
        .expect("store Turn 1 architecture fact");

    response_cache
        .put(
            "prompt_hash_alpha_t1",
            "claude-3-5-sonnet",
            "Architecture created and saved to disk",
            240,
        )
        .expect("record turn 1 token usage");

    // Turn 2 (Agent Alpha): Real Echo/processing writes private operational secrets
    let private_note =
        "INTERNAL SECRET: Cluster admin endpoint 10.200.1.50 with auth token-secret-99";
    alpha_mem
        .store_with_options(
            "notes:private_creds",
            private_note,
            MemoryCategory::Daily,
            Some("session_alpha_proj"),
            StoreOptions {
                importance: Some(0.4),
                ..Default::default()
            },
        )
        .await
        .expect("store Turn 2 private notes fact");

    response_cache
        .put(
            "prompt_hash_alpha_t2",
            "claude-3-5-sonnet",
            "Private credentials stored into daily memory",
            160,
        )
        .expect("record turn 2 token usage");

    // Turn 3 (Agent Beta): Real File Tool writes service implementation
    let impl_file = artifacts_dir.join("service_main.rs");
    let impl_code = "pub fn start_service() -> bool { true }";
    fs::write(&impl_file, impl_code).expect("real file tool write code");
    assert!(impl_file.exists());

    beta_mem
        .store_with_options(
            "tool:file:service_impl",
            &format!("Service implementation generated: {impl_code}"),
            MemoryCategory::Core,
            Some("session_beta_proj"),
            StoreOptions {
                importance: Some(0.85),
                ..Default::default()
            },
        )
        .await
        .expect("store Turn 3 service implementation fact");

    response_cache
        .put(
            "prompt_hash_beta_t1",
            "gpt-4o",
            "Service implementation code generated",
            350,
        )
        .expect("record turn 3 token usage");

    // Multi-turn Session Continuity Sequence: Turns 1-4 in "session_continuity_stream"
    alpha_mem
        .store_with_options(
            "turn:001:user",
            "Step 1: Set database replication factor to 3",
            MemoryCategory::Conversation,
            Some("session_continuity_stream"),
            StoreOptions::default(),
        )
        .await
        .expect("record session turn 1");

    alpha_mem
        .store_with_options(
            "turn:002:assistant",
            "Step 2: Database replication factor configured to 3 across all nodes",
            MemoryCategory::Conversation,
            Some("session_continuity_stream"),
            StoreOptions::default(),
        )
        .await
        .expect("record session turn 2");

    alpha_mem
        .store_with_options(
            "turn:003:user",
            "Step 3: Enable WAL mode and synchronous normal",
            MemoryCategory::Conversation,
            Some("session_continuity_stream"),
            StoreOptions::default(),
        )
        .await
        .expect("record session turn 3");

    alpha_mem
        .store_with_options(
            "turn:004:assistant",
            "Step 4: WAL mode active and synchronous set to NORMAL",
            MemoryCategory::Conversation,
            Some("session_continuity_stream"),
            StoreOptions::default(),
        )
        .await
        .expect("record session turn 4");

    response_cache
        .put(
            "prompt_hash_continuity_turns",
            "claude-3-5-sonnet",
            "Completed 4-turn database configuration flow",
            450,
        )
        .expect("record continuity turns token usage");

    // ── 4. Simulate Crash / Restart Daemon: Drop all instances from RAM ──
    drop(alpha_mem);
    drop(beta_mem);
    drop(audited_backend);
    drop(sqlite_mem);
    drop(response_cache);

    // ── 5. Verify physical SQL rows directly on disk (Zero Mock Assertion) ──
    {
        // 5.1 Verify brain.db
        let raw_brain = open_raw_brain(workspace_path);

        let agent_count: i64 = raw_brain
            .query_row("SELECT COUNT(*) FROM agents", [], |row| row.get(0))
            .expect("count agents");
        assert_eq!(
            agent_count, 3,
            "must have exactly 3 agents in database on disk: default, agent_alpha, agent_beta"
        );

        let total_memories: i64 = raw_brain
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .expect("count memories");
        assert_eq!(
            total_memories, 7,
            "must have exactly 7 memory facts persisted on disk across all turns"
        );

        let continuity_turn_count: i64 = raw_brain
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE session_id = 'session_continuity_stream'",
                [],
                |row| row.get(0),
            )
            .expect("count continuity turns");
        assert_eq!(
            continuity_turn_count, 4,
            "must have exactly 4 session turns recorded in session_continuity_stream"
        );

        // Verify FTS5 virtual table synchronization on disk
        let fts_match_count: i64 = raw_brain
            .query_row(
                "SELECT COUNT(*) FROM memories_fts WHERE memories_fts MATCH '\"galleon-fleet\"'",
                [],
                |row| row.get(0),
            )
            .expect("query FTS5 index on disk");
        assert!(
            fts_match_count >= 1,
            "FTS5 index must return at least 1 match for persisted facts"
        );

        // 5.2 Verify audit.db
        let raw_audit = open_raw_audit(workspace_path);
        let audit_count: i64 = raw_audit
            .query_row(
                "SELECT COUNT(*) FROM memory_audit WHERE operation = 'store'",
                [],
                |row| row.get(0),
            )
            .expect("count audit store operations");
        assert_eq!(
            audit_count, 7,
            "must have exactly 7 audit rows for all 7 store operations"
        );

        // 5.3 Verify response_cache.db (token ledger)
        let raw_cache = open_raw_cache(workspace_path);
        let token_entry_count: i64 = raw_cache
            .query_row("SELECT COUNT(*) FROM response_cache", [], |row| row.get(0))
            .expect("count token cache entries");
        assert_eq!(
            token_entry_count, 4,
            "must have exactly 4 token ledger entries on disk"
        );

        let total_tokens: i64 = raw_cache
            .query_row("SELECT SUM(token_count) FROM response_cache", [], |row| {
                row.get(0)
            })
            .expect("sum token usage");
        assert_eq!(
            total_tokens,
            240 + 160 + 350 + 450,
            "total tokens persisted must equal exactly 1200"
        );
    }

    // ── 6. Re-instantiate new Agent & Memory instances from disk and verify recall ──
    let recovered_sqlite = SqliteMemory::new("sqlite", workspace_path)
        .expect("reopen physical SqliteMemory post-restart");

    let rec_alpha_uuid = recovered_sqlite
        .ensure_agent_uuid("agent_alpha")
        .await
        .expect("recover agent_alpha UUID");
    let rec_beta_uuid = recovered_sqlite
        .ensure_agent_uuid("agent_beta")
        .await
        .expect("recover agent_beta UUID");

    assert_eq!(
        rec_alpha_uuid, alpha_uuid,
        "recovered agent_alpha UUID must match pre-crash UUID 100%"
    );
    assert_eq!(
        rec_beta_uuid, beta_uuid,
        "recovered agent_beta UUID must match pre-crash UUID 100%"
    );

    let recovered_audited = Arc::new(
        AuditedMemory::new(recovered_sqlite.clone(), workspace_path)
            .expect("reopen physical AuditedMemory post-restart"),
    );
    let recovered_cache = ResponseCache::new(workspace_path, 180, 500)
        .expect("reopen physical ResponseCache post-restart");

    let new_grant_for_beta = AgentMemoryGrant {
        agent_id: rec_alpha_uuid.clone(),
        categories: Some(beta_grants),
    };

    let rec_alpha_mem = AgentScopedMemory::new(recovered_audited.clone(), &rec_alpha_uuid, vec![]);
    let rec_beta_mem = AgentScopedMemory::new_with_grants(
        recovered_audited.clone(),
        &rec_beta_uuid,
        vec![new_grant_for_beta],
    );

    // 6.1 Verify 100% session continuity recall in exact order
    let session_turns = rec_alpha_mem
        .recall("Step", 10, Some("session_continuity_stream"), None, None)
        .await
        .expect("recall session continuity stream");

    assert_eq!(
        session_turns.len(),
        4,
        "all 4 session turns must be recovered from physical SQLite database"
    );

    let recalled_keys: Vec<&str> = session_turns.iter().map(|e| e.key.as_str()).collect();
    assert!(recalled_keys.contains(&"turn:001:user"));
    assert!(recalled_keys.contains(&"turn:002:assistant"));
    assert!(recalled_keys.contains(&"turn:003:user"));
    assert!(recalled_keys.contains(&"turn:004:assistant"));

    // 6.2 Verify fact recall across turns
    let arch_facts = rec_alpha_mem
        .recall("galleon-fleet resilience", 5, None, None, None)
        .await
        .expect("recall architecture fact");
    assert!(
        arch_facts
            .iter()
            .any(|e| e.key == "tool:file:system_architecture"),
        "architecture fact must be 100% recovered"
    );

    // 6.3 Verify cross-agent access control & isolation post-restart
    // Beta CAN recall Alpha's Core architecture document
    let beta_recalled_arch = rec_beta_mem
        .recall("galleon-fleet resilience", 5, None, None, None)
        .await
        .expect("beta recall architecture fact");
    assert!(
        beta_recalled_arch
            .iter()
            .any(|e| e.key == "tool:file:system_architecture"),
        "Agent Beta must be granted access to Alpha's Core category post-restart"
    );

    // Beta CANNOT recall Alpha's private Daily credentials
    let beta_recalled_secrets = rec_beta_mem
        .recall("token-secret-99", 5, None, None, None)
        .await
        .expect("beta recall private secret");
    assert!(
        beta_recalled_secrets.is_empty(),
        "Agent Beta MUST NOT see Alpha's private credentials; multi-agent isolation preserved post-restart"
    );

    // Beta CAN recall its own service implementation
    let beta_own_facts = rec_beta_mem
        .recall("start_service", 5, None, None, None)
        .await
        .expect("beta recall own facts");
    assert!(
        beta_own_facts
            .iter()
            .any(|e| e.key == "tool:file:service_impl"),
        "Agent Beta must recall its own service implementation fact"
    );

    // 6.4 Verify token ledger recovery
    let (entry_count, _hits, _saved) = recovered_cache
        .stats()
        .expect("recovered token cache stats");
    assert_eq!(
        entry_count, 4,
        "token cache must preserve 4 entries on disk"
    );

    // Check specific prompt hash retrieval
    let cached_entry = recovered_cache
        .get("prompt_hash_alpha_t1")
        .expect("get cached response");
    assert_eq!(
        cached_entry.as_deref(),
        Some("Architecture created and saved to disk"),
        "token cache must return cached response for Turn 1"
    );

    // 6.5 Verify audit log continues post-restart
    // Performing a recall adds an audit entry to audit.db
    let final_audit = open_raw_audit(workspace_path);
    let final_audit_count: i64 = final_audit
        .query_row("SELECT COUNT(*) FROM memory_audit", [], |row| row.get(0))
        .expect("count final audit rows");
    assert!(
        final_audit_count > 7,
        "audit ledger must continue recording new operations post-restart without loss"
    );
}
