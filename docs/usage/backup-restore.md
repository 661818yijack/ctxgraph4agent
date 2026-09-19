# Backup and Restore ctxgraph Databases

ctxgraph stores its default local graph in a single SQLite database at `.ctxgraph/graph.db`.
That makes backup and restore simple, but the safe rule is: copy files only when no ctxgraph
process is writing to the database.

Use this guide when you want a recoverable checkpoint before experiments, migrations, demos, or
manual database maintenance.

## Safe backup workflow

Run these commands from the project directory that contains `.ctxgraph/`.

```bash
# Confirm the default database exists.
test -f .ctxgraph/graph.db

# Create a timestamped backup directory in UTC.
BACKUP_DIR="backups/ctxgraph-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$BACKUP_DIR"

# Copy the SQLite database without modifying the source.
cp .ctxgraph/graph.db "$BACKUP_DIR/graph.db"

# Verify the backup artifact exists before relying on it.
test -f "$BACKUP_DIR/graph.db"
```

The backup directory is intentionally outside `.ctxgraph/` so normal ctxgraph commands keep using
the active database.

## Restore workflow

**Stop active writers before restoring.** Do not restore over a database while `ctxgraph mcp start`,
a long-running agent, or another ctxgraph command may be writing to `.ctxgraph/graph.db`.

Keep a copy of the current database before replacing it:

```bash
# Save the current database first, even when you believe it is broken.
PRE_RESTORE_DIR="backups/pre-restore-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$PRE_RESTORE_DIR"
cp .ctxgraph/graph.db "$PRE_RESTORE_DIR/graph.db"

# Restore the chosen backup.
cp "$BACKUP_DIR/graph.db" .ctxgraph/graph.db
```

If the restore is not what you expected, the pre-restore copy gives you a fast rollback path:

```bash
cp "$PRE_RESTORE_DIR/graph.db" .ctxgraph/graph.db
```

## Read-only verification

After restoring, run read-only checks before logging new episodes. These checks read existing data; they do not write new episodes.

```bash
ctxgraph stats
ctxgraph query "recent decisions" --limit 5
ctxgraph decisions list --limit 5
```

If these commands return the expected counts and memories, the restored database is ready for normal
use.

## Notes

- For very large or actively shared databases, prefer stopping the writer first rather than racing a
  live file copy.
- Keep backup directories under your normal encrypted or private storage boundary if the graph may
  contain sensitive team context.
- Do not commit `.ctxgraph/graph.db` or backup directories to Git.
