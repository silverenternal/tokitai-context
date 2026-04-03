# P2-010: Multi-Version Concurrency Control (MVCC)

## Overview

This document describes the MVCC (Multi-Version Concurrency Control) implementation for tokitai-context, providing **snapshot isolation** for concurrent transactions.

## Status: ✅ COMPLETE

**Implementation Date**: April 3, 2026
**Location**: `src/mvcc/`
**Tests**: 33/33 passing

---

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                      MvccManager                             │
│  - Transaction ID generation (monotonically increasing)     │
│  - Snapshot ID generation                                    │
│  - Active transaction tracking                               │
│  - Statistics collection                                     │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
┌─────────────────────────┐      ┌─────────────────────────┐
│   TransactionManager    │      │    SnapshotManager      │
│  - Track active txns    │      │  - Track snapshots      │
│  - Commit/abort log     │      │  - Visibility checks    │
└─────────────────────────┘      └─────────────────────────┘
              │                               │
              └───────────────┬───────────────┘
                              │
                              ▼
                  ┌───────────────────────┐
                  │    VersionChain       │
                  │  key → v1 → v2 → v3   │
                  └───────────────────────┘
```

### Module Structure

```
src/mvcc/
├── mod.rs              # MvccManager, configuration, statistics
├── transaction.rs      # Transaction, TransactionManager
├── snapshot.rs         # Snapshot, SnapshotManager
└── version_chain.rs    # Version, VersionChain, VersionChainRegistry
```

---

## Key Concepts

### Snapshot Isolation

Snapshot isolation guarantees:

1. **Consistent Reads**: All reads in a transaction see a consistent snapshot of the data
2. **Non-blocking Reads**: Readers never block writers
3. **Non-blocking Writes**: Writers never block readers
4. **Read Your Own Writes**: Transactions see their own uncommitted writes

### Version Chain

Each key maintains a linked list of versions:

```
Key: "user:123"
│
▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Version 3  │───▶│  Version 2  │───▶│  Version 1  │
│  txn_id: 5  │    │  txn_id: 3  │    │  txn_id: 1  │
│  value: C   │    │  value: B   │    │  value: A   │
│  deleted: F │    │  deleted: F │    │  deleted: T │
└─────────────┘    └─────────────┘    └─────────────┘
     (latest)                              (oldest)
```

### Visibility Rules

A version is visible to a snapshot if:

1. `version.txn_id < snapshot.id` (created before snapshot)
2. `version.txn_id ∉ snapshot.active_set` (not in active transactions at snapshot time)

---

## API Reference

### MvccManager

```rust
use tokitai_context::mvcc::{MvccManager, MvccConfig};

let config = MvccConfig::default();
let manager = MvccManager::new(config);

// Begin a read-write transaction
let mut txn = manager.begin_rw_transaction();

// Begin a read-only snapshot
let mut snapshot = manager.begin_snapshot();

// Commit/abort transactions
manager.commit_transaction(&mut txn)?;
manager.abort_transaction(&mut txn)?;

// Release snapshots
manager.release_snapshot(&mut snapshot)?;
```

### Transaction

```rust
use tokitai_context::mvcc::Transaction;

// Buffer writes (not visible until commit)
txn.put("key1".to_string(), b"value1".to_vec());
txn.delete("key2".to_string());

// Access transaction metadata
let id = txn.id();              // TransactionId
let state = txn.state();        // TransactionState
let writes = txn.get_writes();  // &HashMap<String, WriteOperation>
```

### Snapshot

```rust
use tokitai_context::mvcc::Snapshot;

// Access snapshot metadata
let id = snapshot.id();                     // SnapshotId
let active = snapshot.active_transactions(); // &[TransactionId]
let visible = snapshot.is_visible(txn_id);   // bool
```

### VersionChain

```rust
use tokitai_context::mvcc::VersionChain;

// Create and modify version chains
let chain = VersionChain::new("key1".to_string(), Some(stats));
chain.put(1, b"value".to_vec());
chain.delete(2);

// Query versions
let latest = chain.get_latest();
let visible = chain.get_visible(|txn_id| txn_id < 5 && !active.contains(&txn_id));
let all = chain.get_all_versions();

// Garbage collection
let collected = chain.garbage_collect(min_visible_txn_id, max_versions);
```

---

## Configuration

### MvccConfig

```rust
pub struct MvccConfig {
    /// Maximum number of versions to keep per key (for GC)
    pub max_versions_per_key: usize,  // default: 10
    
    /// Enable automatic garbage collection
    pub enable_auto_gc: bool,  // default: true
    
    /// GC threshold (number of versions before triggering GC)
    pub gc_threshold: usize,  // default: 5
    
    /// Snapshot timeout in milliseconds (0 = no timeout)
    pub snapshot_timeout_ms: u64,  // default: 60_000 (1 minute)
}
```

---

## Usage Examples

### Basic Transaction

```rust
use tokitai_context::mvcc::{MvccManager, MvccConfig};

let manager = MvccManager::new(MvccConfig::default());

// Start transaction
let mut txn = manager.begin_rw_transaction();
let txn_id = txn.id();

// Perform writes (buffered)
txn.put("user:1".to_string(), b"Alice".to_vec());
txn.put("user:2".to_string(), b"Bob".to_vec());

// Commit (makes writes visible)
manager.commit_transaction(&mut txn)?;
```

### Snapshot Read

```rust
// Start a transaction and write
let mut txn1 = manager.begin_rw_transaction();
txn1.put("key".to_string(), b"value1".to_vec());
manager.commit_transaction(&mut txn1)?;

// Create a snapshot
let mut snapshot = manager.begin_snapshot();
let snapshot_id = snapshot.id();

// Another transaction writes
let mut txn2 = manager.begin_rw_transaction();
txn2.put("key".to_string(), b"value2".to_vec());
manager.commit_transaction(&mut txn2)?;

// Snapshot still sees the old value
let chain = registry.get_or_create("key");
let visible = chain.get_visible(|txn_id| snapshot.is_visible(txn_id));
assert_eq!(visible.unwrap().value_bytes(), Some(b"value1".as_slice()));
```

### Version Garbage Collection

```rust
let stats = Arc::new(MvccStats::default());
let chain = VersionChain::new("key".to_string(), Some(stats.clone()));

// Add 10 versions
for i in 1..=10 {
    chain.put(i, format!("v{}", i).into_bytes());
}

// GC: keep versions >= 5, max 5 versions
let collected = chain.garbage_collect(5, 5);
assert_eq!(collected, 5);
assert_eq!(chain.version_count(), 5);

// Kept versions: 10, 9, 8, 7, 6 (newest 5 that are >= 5)
```

---

## Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|----------------|-------|
| `begin_rw_transaction()` | O(1) | Atomic ID generation |
| `begin_snapshot()` | O(A) | A = active transactions |
| `commit_transaction()` | O(1) | Amortized |
| `get_visible()` | O(V) | V = versions per key |
| `garbage_collect()` | O(V) | V = total versions |

### Memory Overhead

- **Per Version**: ~64 bytes (txn_id, value, timestamp, next pointer)
- **Per Snapshot**: ~100 bytes + active transaction set
- **Per Transaction**: ~50 bytes + write buffer

---

## Statistics

```rust
let stats = manager.stats().snapshot();

// Transaction metrics
println!("Transactions started: {}", stats.transactions_started);
println!("Transactions committed: {}", stats.transactions_committed);
println!("Transactions aborted: {}", stats.transactions_aborted);
println!("Active transactions: {}", stats.active_transactions);

// Snapshot metrics
println!("Snapshots created: {}", stats.snapshots_created);
println!("Snapshots released: {}", stats.snapshots_released);
println!("Active snapshots: {}", stats.active_snapshots);

// Version metrics
println!("Versions created: {}", stats.versions_created);
println!("Versions GC collected: {}", stats.versions_gc_collected);
```

---

## Testing

### Run Tests

```bash
# Run all MVCC tests
cargo test --lib mvcc

# Run specific test categories
cargo test --lib mvcc::transaction
cargo test --lib mvcc::snapshot
cargo test --lib mvcc::version_chain
```

### Test Coverage

- **Transaction Tests**: 8 tests
  - Creation, state transitions, write buffering
  - Manager lifecycle, history limits
  - Concurrent transaction IDs

- **Snapshot Tests**: 8 tests
  - Creation, visibility, read counting
  - Manager lifecycle, statistics
  - Concurrent snapshots

- **VersionChain Tests**: 10 tests
  - Version creation, append, delete
  - Visibility checks, garbage collection
  - Registry operations, statistics

- **Manager Tests**: 7 tests
  - Manager creation, transaction lifecycle
  - Snapshot lifecycle, visibility rules
  - Abort handling, statistics tracking

---

## Integration Points

### Future Integration with FileKV

The MVCC module is designed to integrate with FileKV:

```rust
// Future integration example
pub struct FileKV {
    // ... existing fields ...
    mvcc_manager: MvccManager,
    version_registry: VersionChainRegistry,
}

impl FileKV {
    pub fn get_with_snapshot(&self, key: &str, snapshot: &Snapshot) -> Result<Option<Vec<u8>>> {
        let chain = self.version_registry.get_or_create(key);
        let visible = chain.get_visible(|txn_id| snapshot.is_visible(txn_id));
        Ok(visible.and_then(|v| v.value_bytes().map(|b| b.to_vec())))
    }
    
    pub fn put_with_transaction(&self, txn: &mut Transaction, key: String, value: Vec<u8>) {
        txn.put(key, value);
        // Writes are buffered until commit
    }
}
```

---

## Limitations and Future Work

### Current Limitations

1. **No Persistent Storage**: Versions are in-memory only
2. **No Write Conflict Detection**: Last-write-wins semantics
3. **No Long-running Transaction Protection**: Old versions retained indefinitely
4. **No Serializable Isolation**: Only snapshot isolation provided

### Future Enhancements

1. **Persistent Version Chains**: Store version history in segments
2. **Write-Write Conflict Detection**: Detect and prevent lost updates
3. **Snapshot Timeout**: Automatically release old snapshots
4. **Serializable Snapshot Isolation (SSI)**: Stronger consistency guarantee
5. **Multi-Snapshot GC**: Track minimum visible txn across all snapshots

---

## References

- [Snapshot Isolation Wikipedia](https://en.wikipedia.org/wiki/Snapshot_isolation)
- [MVCC in Databases](https://www.cockroachlabs.com/blog/how-better-timestamps-solve-the-transaction-latency-problem-in-distributed-databases/)
- [PostgreSQL MVCC](https://www.postgresql.org/docs/current/mvcc-intro.html)

---

## Conclusion

The MVCC implementation provides a solid foundation for snapshot isolation in tokitai-context. With 33 passing tests and a clean API, it's ready for integration with the FileKV storage engine.

**Next Steps**:
1. Integrate MVCC into FileKV read/write paths
2. Add persistent version storage
3. Implement automatic GC triggering
4. Add conflict detection for write-write conflicts
