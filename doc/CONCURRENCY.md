# Concurrency Model

This document describes the concurrency guarantees and locking strategies used in tokitai-context.

## Overview

Tokitai-context is designed for concurrent access with the following principles:

1. **Session-level isolation**: Different sessions can be accessed concurrently without blocking
2. **Layer-level locking**: Within a session, different layers (transient, short-term, long-term) can be accessed concurrently
3. **Read-Write separation**: Multiple readers allowed, writers get exclusive access
4. **Copy-on-Write**: Fork operations use O(1) symlinks without locking source data

## Locking Strategy

### Session Cache

```rust
sessions: HashMap<String, SessionContext>
```

- **Lock type**: `parking_lot::RwLock`
- **Read lock**: Getting existing session
- **Write lock**: Creating new session
- **Granularity**: Per-operation (lock held only during HashMap access)

### Hash Index

```rust
hash_index: HashIndex
```

- **Lock type**: Internal `RwLock`
- **Read lock**: `get_path()`, `contains()`, `list_hashes()`
- **Write lock**: `add()`, `remove()`
- **Granularity**: Global (single lock for all hashes)

### COW Manager

```rust
symlinks: Arc<RwLock<HashMap<PathBuf, SymlinkMetadata>>>
```

- **Lock type**: `Arc<RwLock<>>` for thread-safe shared ownership
- **Read lock**: Checking symlink metadata
- **Write lock**: Creating new symlinks, marking as written
- **Granularity**: Per-file tracking

### Layer Access

Within a session, layers are independent:

```rust
struct SessionContext {
    transient: TransientLayer,
    short_term: ShortTermLayer,
    long_term: LongTermLayer,
}
```

- **No inter-layer locking**: Layers can be accessed concurrently
- **Intra-layer locking**: Each layer manages its own internal state

## Thread Safety Guarantees

### Send + Sync

All public types implement `Send + Sync` where applicable:

- `Context`: `Send + Sync` (uses internal `Arc<RwLock<>>` for shared state)
- `ParallelContextManager`: `Send` (not `Sync` - use one per thread or wrap in `Arc<Mutex<>>`)
- `FileContextServiceImpl`: `Send` (session cache requires exclusive access)

### Atomic Operations

The following operations are atomic:

1. **Fork (COW)**: Symlink creation is atomic at filesystem level
2. **Hash index add**: Protected by `RwLock`
3. **Log append**: File append is atomic on Unix

### Non-Atomic Operations

These operations are NOT atomic and may leave intermediate state:

1. **Merge**: Multi-step process (read source, write target, update graph)
2. **Session cleanup**: Deletes multiple files/directories

For non-atomic operations, use WAL for recovery.

## Concurrency Patterns

### Pattern 1: Multiple Sessions Concurrently

```rust
// Safe - different sessions don't block each other
let mut ctx1 = Context::open("./.context")?;
let mut ctx2 = Context::open("./.context")?;

ctx1.store("session-1", data1, Layer::ShortTerm)?;
ctx2.store("session-2", data2, Layer::ShortTerm)?;
// These operations run in parallel
```

### Pattern 2: Read-Many-Write-One

```rust
// Multiple readers can access same session
let ctx = Context::open("./.context")?;
let content1 = ctx.retrieve("session-1", hash1)?;
let content2 = ctx.retrieve("session-1", hash2)?;
// These reads don't block each other

// Writer needs exclusive access to that session
ctx.delete("session-1", hash1)?;
// This blocks other operations on session-1
```

### Pattern 3: Parallel Branch Operations

```rust
// Different branches can be merged concurrently
manager.merge("feature-1", "main", None)?;
manager.merge("feature-2", "main", None)?;
// Safe - different source branches

// Same target branch needs coordination
// Use a mutex if merging into same target from multiple threads
```

## Deadlock Prevention

The library prevents deadlocks through:

1. **Lock ordering**: Always acquire locks in order: session → layer → index
2. **Short lock duration**: Locks held only for duration of single operation
3. **No nested locks**: Operations don't acquire multiple locks simultaneously

## Performance Considerations

### Lock Contention

High contention scenarios:

- **Hash index**: All writes go through single lock
  - Mitigation: Batch writes, use `add_batch()` when available
  - Future: Shard hash index by hash prefix

- **Session cache**: Frequent session creation/deletion
  - Mitigation: Reuse sessions, avoid rapid create/delete cycles

### Scalability

Expected scaling:

| Scenario | Scaling | Notes |
|----------|---------|-------|
| Multiple sessions | Excellent | No inter-session locking |
| Multiple readers | Excellent | RwLock allows concurrent reads |
| Multiple writers (same session) | Poor | Serialized by design |
| Multiple writers (different sessions) | Good | Only hash index is bottleneck |

## Windows Considerations

On Windows, symlink support is limited:

- **Junction points**: Used instead of symlinks for directories
- **File copy**: Used as fallback for files
- **Performance**: 2-3x slower fork operations

The COW manager automatically detects platform capabilities and uses the best available strategy.

## Error Recovery

After a crash or panic:

1. **Call `Context::recover()`**: Scans for inconsistencies
2. **Check WAL**: Incomplete operations are logged
3. **Manual intervention**: For severe corruption, see `doc/RECOVERY.md`

## Best Practices

1. **Use one `Context` per thread**: Avoid sharing across threads
2. **For shared access, use `Arc<Mutex<Context>>`**: Explicit locking
3. **Batch operations**: Reduce lock acquisition overhead
4. **Clean up sessions**: Prevent unbounded growth
5. **Enable WAL for production**: Critical for crash recovery

## Future Improvements

Planned enhancements:

- [ ] Sharded hash index for better write parallelism
- [ ] Per-session WAL for independent recovery
- [ ] Async support with `tokio::sync::RwLock`
- [ ] Lock-free data structures for hot paths
