use rusqlite::{params, Connection};

use crate::error::Result;

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_initial",
        r#"
    -- Episodes: raw events
    CREATE TABLE IF NOT EXISTS episodes (
        id          TEXT PRIMARY KEY,
        content     TEXT NOT NULL,
        source      TEXT,
        recorded_at TEXT NOT NULL,
        metadata    TEXT,
        embedding   BLOB
    );

    -- Entities: extracted nodes
    CREATE TABLE IF NOT EXISTS entities (
        id          TEXT PRIMARY KEY,
        name        TEXT NOT NULL,
        entity_type TEXT NOT NULL,
        summary     TEXT,
        created_at  TEXT NOT NULL,
        metadata    TEXT
    );

    -- Edges: relationships between entities
    CREATE TABLE IF NOT EXISTS edges (
        id          TEXT PRIMARY KEY,
        source_id   TEXT NOT NULL REFERENCES entities(id),
        target_id   TEXT NOT NULL REFERENCES entities(id),
        relation    TEXT NOT NULL,
        fact        TEXT,
        valid_from  TEXT,
        valid_until TEXT,
        recorded_at TEXT NOT NULL,
        confidence  REAL DEFAULT 1.0,
        episode_id  TEXT REFERENCES episodes(id),
        metadata    TEXT
    );

    -- Episode-Entity junction table
    CREATE TABLE IF NOT EXISTS episode_entities (
        episode_id  TEXT REFERENCES episodes(id),
        entity_id   TEXT REFERENCES entities(id),
        span_start  INTEGER,
        span_end    INTEGER,
        PRIMARY KEY (episode_id, entity_id)
    );

    -- Entity aliases for deduplication
    CREATE TABLE IF NOT EXISTS aliases (
        canonical_id TEXT REFERENCES entities(id),
        alias_name   TEXT NOT NULL,
        similarity   REAL,
        UNIQUE(canonical_id, alias_name)
    );

    -- Community clusters
    CREATE TABLE IF NOT EXISTS communities (
        id          TEXT PRIMARY KEY,
        summary     TEXT,
        entity_ids  TEXT,
        created_at  TEXT NOT NULL,
        updated_at  TEXT
    );

    -- FTS5 indexes
    CREATE VIRTUAL TABLE IF NOT EXISTS episodes_fts USING fts5(
        content, source, metadata,
        content=episodes, content_rowid=rowid
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
        name, entity_type, summary,
        content=entities, content_rowid=rowid
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS edges_fts USING fts5(
        fact, relation,
        content=edges, content_rowid=rowid
    );

    -- Performance indexes
    CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
    CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id);
    CREATE INDEX IF NOT EXISTS idx_edges_relation ON edges(relation);
    CREATE INDEX IF NOT EXISTS idx_edges_valid ON edges(valid_from, valid_until);
    CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
    CREATE INDEX IF NOT EXISTS idx_episode_entities ON episode_entities(entity_id);
    CREATE INDEX IF NOT EXISTS idx_episodes_source ON episodes(source);
    CREATE INDEX IF NOT EXISTS idx_episodes_recorded ON episodes(recorded_at);

    -- FTS5 triggers: keep indexes in sync
    CREATE TRIGGER IF NOT EXISTS episodes_ai AFTER INSERT ON episodes BEGIN
        INSERT INTO episodes_fts(rowid, content, source, metadata)
        VALUES (new.rowid, new.content, new.source, new.metadata);
    END;

    CREATE TRIGGER IF NOT EXISTS episodes_ad AFTER DELETE ON episodes BEGIN
        INSERT INTO episodes_fts(episodes_fts, rowid, content, source, metadata)
        VALUES ('delete', old.rowid, old.content, old.source, old.metadata);
    END;

    CREATE TRIGGER IF NOT EXISTS episodes_au AFTER UPDATE ON episodes BEGIN
        INSERT INTO episodes_fts(episodes_fts, rowid, content, source, metadata)
        VALUES ('delete', old.rowid, old.content, old.source, old.metadata);
        INSERT INTO episodes_fts(rowid, content, source, metadata)
        VALUES (new.rowid, new.content, new.source, new.metadata);
    END;

    CREATE TRIGGER IF NOT EXISTS entities_ai AFTER INSERT ON entities BEGIN
        INSERT INTO entities_fts(rowid, name, entity_type, summary)
        VALUES (new.rowid, new.name, new.entity_type, new.summary);
    END;

    CREATE TRIGGER IF NOT EXISTS entities_ad AFTER DELETE ON entities BEGIN
        INSERT INTO entities_fts(entities_fts, rowid, name, entity_type, summary)
        VALUES ('delete', old.rowid, old.name, old.entity_type, old.summary);
    END;

    CREATE TRIGGER IF NOT EXISTS entities_au AFTER UPDATE ON entities BEGIN
        INSERT INTO entities_fts(entities_fts, rowid, name, entity_type, summary)
        VALUES ('delete', old.rowid, old.name, old.entity_type, old.summary);
        INSERT INTO entities_fts(rowid, name, entity_type, summary)
        VALUES (new.rowid, new.name, new.entity_type, new.summary);
    END;

    CREATE TRIGGER IF NOT EXISTS edges_ai AFTER INSERT ON edges BEGIN
        INSERT INTO edges_fts(rowid, fact, relation)
        VALUES (new.rowid, new.fact, new.relation);
    END;

    CREATE TRIGGER IF NOT EXISTS edges_ad AFTER DELETE ON edges BEGIN
        INSERT INTO edges_fts(edges_fts, rowid, fact, relation)
        VALUES ('delete', old.rowid, old.fact, old.relation);
    END;

    CREATE TRIGGER IF NOT EXISTS edges_au AFTER UPDATE ON edges BEGIN
        INSERT INTO edges_fts(edges_fts, rowid, fact, relation)
        VALUES ('delete', old.rowid, old.fact, old.relation);
        INSERT INTO edges_fts(rowid, fact, relation)
        VALUES (new.rowid, new.fact, new.relation);
    END;
    "#,
    ),
    (
        "002_entity_embeddings",
        r#"
    -- Add embedding column to entities table (episodes already has it from 001)
    -- We use a Rust-side check since SQLite ALTER TABLE ADD COLUMN is not idempotent
    "#,
    ),
    (
        "003_memory_type_and_ttl",
        r#"
    -- NOTE: This SQL is not executed for version 003; the Rust-side idempotent path
    -- (below) handles it instead. Keep this in sync or remove in a future cleanup.
    -- Add memory_type and ttl_seconds to entities
    ALTER TABLE entities ADD COLUMN memory_type TEXT NOT NULL DEFAULT 'Fact';
    ALTER TABLE entities ADD COLUMN ttl_seconds INTEGER;

    -- Add memory_type and ttl_seconds to edges
    ALTER TABLE edges ADD COLUMN memory_type TEXT NOT NULL DEFAULT 'Fact';
    ALTER TABLE edges ADD COLUMN ttl_seconds INTEGER;

    -- Set default TTLs for existing rows (only where ttl_seconds IS NULL for idempotency)
    UPDATE entities SET ttl_seconds = 7776000 WHERE ttl_seconds IS NULL;
        -- 7776000 = 90 days in seconds (Fact default)
    UPDATE edges SET ttl_seconds = 7776000 WHERE ttl_seconds IS NULL;

    -- Index for TTL cleanup queries (used by A6)
    CREATE INDEX IF NOT EXISTS idx_entities_memory_type ON entities(memory_type);
    CREATE INDEX IF NOT EXISTS idx_entities_created_at ON entities(created_at);
    "#,
    ),
    (
        "004_usage_count_and_last_recalled_at",
        r#"
    -- NOTE: This SQL is not executed for version 004; the Rust-side idempotent path
    -- (below) handles it instead. Keep this in sync or remove in a future cleanup.
    ALTER TABLE entities ADD COLUMN usage_count INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE entities ADD COLUMN last_recalled_at TEXT;
    ALTER TABLE edges ADD COLUMN usage_count INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE edges ADD COLUMN last_recalled_at TEXT;
    "#,
    ),
    (
        "006_episode_compression",
        r#"
    -- NOTE: This SQL is not executed for version 006; the Rust-side idempotent path
    -- (below) handles it instead. Keep this in sync or remove in a future cleanup.
    ALTER TABLE episodes ADD COLUMN compression_id TEXT;
    ALTER TABLE episodes ADD COLUMN memory_type TEXT NOT NULL DEFAULT 'experience';
    CREATE INDEX IF NOT EXISTS idx_episodes_compression_id ON episodes(compression_id);
    CREATE INDEX IF NOT EXISTS idx_episodes_memory_type ON episodes(memory_type);
    "#,
    ),
    (
        "007_skills_table",
        r#"
    CREATE TABLE IF NOT EXISTS skills (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT NOT NULL,
        trigger_condition TEXT NOT NULL,
        action TEXT NOT NULL,
        success_count INTEGER NOT NULL DEFAULT 0,
        failure_count INTEGER NOT NULL DEFAULT 0,
        confidence REAL NOT NULL DEFAULT 0.0,
        superseded_by TEXT,
        created_at TEXT NOT NULL,
        entity_types TEXT NOT NULL DEFAULT '[]',
        provenance TEXT,
        scope TEXT NOT NULL DEFAULT 'private',
        created_by_agent TEXT NOT NULL DEFAULT ''
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS skills_fts USING fts5(skill_id, name, description);

    CREATE INDEX IF NOT EXISTS idx_skills_superseded ON skills(superseded_by);
    CREATE INDEX IF NOT EXISTS idx_skills_scope ON skills(scope);
    CREATE INDEX IF NOT EXISTS idx_skills_agent ON skills(created_by_agent);

    -- FTS5 triggers to keep skills_fts in sync
    CREATE TRIGGER IF NOT EXISTS skills_ai AFTER INSERT ON skills BEGIN
        INSERT INTO skills_fts(skill_id, name, description)
        VALUES (new.id, new.name, new.description);
    END;

    CREATE TRIGGER IF NOT EXISTS skills_ad AFTER DELETE ON skills BEGIN
        INSERT INTO skills_fts(skills_fts, rowid, skill_id, name, description)
        VALUES ('delete', old.rowid, old.id, old.name, old.description);
    END;

    CREATE TRIGGER IF NOT EXISTS skills_au AFTER UPDATE ON skills BEGIN
        INSERT INTO skills_fts(skills_fts, rowid, skill_id, name, description)
        VALUES ('delete', old.rowid, old.id, old.name, old.description);
        INSERT INTO skills_fts(skill_id, name, description)
        VALUES (new.id, new.name, new.description);
    END;
    "#,
    ),
    (
        "008_fts5_candidate_retrieval",
        r#"
    -- NOTE: FTS5 virtual tables (episodes_fts, entities_fts, edges_fts) and their
    -- triggers were created in Migration 001. This migration is a marker for A4a
    -- (FTS5 + graph candidate retrieval) and performs no schema changes.
    "#,
    ),
    (
        "009_system_metadata",
        r#"
    -- NOTE: system_metadata table for last_cleanup_at and cleanup_in_progress flag.
    -- Implemented via Rust-side idempotent path (below) for safe partial-migration recovery.
    "#,
    ),
];

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    for (version, sql) in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM _migrations WHERE version = ?1)",
            [version],
            |row| row.get(0),
        )?;

        if !already_applied {
            // Migration 002: add embedding column to entities if not present
            if *version == "002_entity_embeddings" {
                let has_col: bool = {
                    let mut col_stmt = conn.prepare(
                        "SELECT COUNT(*) FROM pragma_table_info('entities') WHERE name = 'embedding'",
                    )?;
                    col_stmt
                        .query_row([], |row| row.get::<_, i64>(0))
                        .map(|n| n > 0)?
                };
                if !has_col {
                    conn.execute_batch("ALTER TABLE entities ADD COLUMN embedding BLOB;")?;
                }
            } else if *version == "003_memory_type_and_ttl" {
                // Check each column independently on BOTH tables for safe partial-migration recovery.
                // If interrupted mid-ALTER (e.g. entities gets columns but edges doesn't),
                // reopening should add only the missing columns.
                fn column_exists(conn: &Connection, table: &str, col: &str) -> Result<bool> {
                    let mut stmt =
                        conn.prepare("SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2")?;
                    stmt.query_row(params![table, col], |row| row.get::<_, i64>(0))
                        .map(|n| n > 0)
                        .map_err(Into::into)
                }

                if !column_exists(conn, "entities", "memory_type")? {
                    conn.execute_batch(
                        "ALTER TABLE entities ADD COLUMN memory_type TEXT NOT NULL DEFAULT 'Fact';",
                    )?;
                }
                if !column_exists(conn, "entities", "ttl_seconds")? {
                    conn.execute_batch("ALTER TABLE entities ADD COLUMN ttl_seconds INTEGER;")?;
                }
                if !column_exists(conn, "edges", "memory_type")? {
                    conn.execute_batch(
                        "ALTER TABLE edges ADD COLUMN memory_type TEXT NOT NULL DEFAULT 'Fact';",
                    )?;
                }
                if !column_exists(conn, "edges", "ttl_seconds")? {
                    conn.execute_batch("ALTER TABLE edges ADD COLUMN ttl_seconds INTEGER;")?;
                }

                // Backfill memory_type from entity_type for known types so pre-A1 rows
                // with entity_type='Decision' etc. get the correct memory_type instead of 'Fact'.
                conn.execute_batch(
                    "UPDATE entities SET memory_type = LOWER(entity_type)
                     WHERE LOWER(entity_type) IN ('decision', 'pattern', 'experience', 'preference');",
                )?;

                // Set per-type TTL defaults for existing rows (idempotent).
                // Pattern: no TTL (NULL)
                conn.execute_batch(
                    "UPDATE entities SET ttl_seconds = NULL WHERE LOWER(entity_type) IN ('pattern') AND ttl_seconds IS NOT NULL;",
                )?;
                // Experience: 14 days
                conn.execute_batch(
                    "UPDATE entities SET ttl_seconds = 1209600 WHERE LOWER(entity_type) IN ('experience') AND (ttl_seconds IS NULL OR ttl_seconds = 7776000);",
                )?;
                // Preference: 30 days
                conn.execute_batch(
                    "UPDATE entities SET ttl_seconds = 2592000 WHERE LOWER(entity_type) IN ('preference') AND (ttl_seconds IS NULL OR ttl_seconds = 7776000);",
                )?;
                // Fact/Decision/Unknown: 90 days (default)
                conn.execute_batch(
                    "UPDATE entities SET ttl_seconds = 7776000 WHERE ttl_seconds IS NULL;
                     UPDATE edges SET ttl_seconds = 7776000 WHERE ttl_seconds IS NULL;",
                )?;

                // Indexes for lifecycle/cleanup queries (A6 and beyond)
                conn.execute_batch(
                    "CREATE INDEX IF NOT EXISTS idx_entities_memory_type ON entities(memory_type);
                     CREATE INDEX IF NOT EXISTS idx_entities_created_at ON entities(created_at);
                     CREATE INDEX IF NOT EXISTS idx_edges_memory_type ON edges(memory_type);
                     CREATE INDEX IF NOT EXISTS idx_edges_recorded_at ON edges(recorded_at);",
                )?;
            } else if *version == "004_usage_count_and_last_recalled_at" {
                // Check each column independently on BOTH tables for safe partial-migration recovery.
                fn column_exists(conn: &Connection, table: &str, col: &str) -> Result<bool> {
                    let mut stmt =
                        conn.prepare("SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2")?;
                    stmt.query_row(params![table, col], |row| row.get::<_, i64>(0))
                        .map(|n| n > 0)
                        .map_err(Into::into)
                }

                if !column_exists(conn, "entities", "usage_count")? {
                    conn.execute_batch(
                        "ALTER TABLE entities ADD COLUMN usage_count INTEGER NOT NULL DEFAULT 0;",
                    )?;
                }
                if !column_exists(conn, "entities", "last_recalled_at")? {
                    conn.execute_batch("ALTER TABLE entities ADD COLUMN last_recalled_at TEXT;")?;
                }
                if !column_exists(conn, "edges", "usage_count")? {
                    conn.execute_batch(
                        "ALTER TABLE edges ADD COLUMN usage_count INTEGER NOT NULL DEFAULT 0;",
                    )?;
                }
                if !column_exists(conn, "edges", "last_recalled_at")? {
                    conn.execute_batch("ALTER TABLE edges ADD COLUMN last_recalled_at TEXT;")?;
                }
            } else if *version == "006_episode_compression" {
                fn column_exists(conn: &Connection, table: &str, col: &str) -> Result<bool> {
                    let mut stmt =
                        conn.prepare("SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2")?;
                    stmt.query_row(params![table, col], |row| row.get::<_, i64>(0))
                        .map(|n| n > 0)
                        .map_err(Into::into)
                }

                if !column_exists(conn, "episodes", "compression_id")? {
                    conn.execute_batch("ALTER TABLE episodes ADD COLUMN compression_id TEXT;")?;
                }
                if !column_exists(conn, "episodes", "memory_type")? {
                    conn.execute_batch(
                        "ALTER TABLE episodes ADD COLUMN memory_type TEXT NOT NULL DEFAULT 'experience';",
                    )?;
                }

                // Indexes for compression queries
                conn.execute_batch(
                    "CREATE INDEX IF NOT EXISTS idx_episodes_compression_id ON episodes(compression_id);
                     CREATE INDEX IF NOT EXISTS idx_episodes_memory_type ON episodes(memory_type);",
                )?;
            } else if *version == "009_system_metadata" {
                // Create system_metadata table if not exists (idempotent)
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS system_metadata (
                        key TEXT PRIMARY KEY,
                        value TEXT NOT NULL
                    );",
                )?;
            } else {
                conn.execute_batch(sql)?;
            }
            conn.execute(
                "INSERT INTO _migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![version, chrono::Utc::now().to_rfc3339()],
            )?;
        }
    }

    Ok(())
}
