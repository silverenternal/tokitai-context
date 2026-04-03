# Migration Guide: Tokitai-Context v0.1.0 → v0.2.0

This guide helps you migrate from the old API to the new simplified Facade API introduced in v0.2.0.

## What's New in v0.2.0

### Major Changes

1. **New Facade API**: Simplified high-level interface for common operations
2. **Write-Ahead Log (WAL)**: Automatic crash recovery (enabled by default)
3. **Modular Architecture**: Reorganized into core/, parallel/, optimization/, ai/ layers
4. **Better Error Handling**: Improved error messages and recovery mechanisms
5. **Enhanced Tracing**: Full instrumentation with `#[tracing::instrument]`

### Breaking Changes

- Module reorganization (old paths still work via re-exports)
- `ContextRoot` API unchanged but now also available via `Context::open()`
- Feature flags updated: added `wal`, `core`; defaults now include `wal`

## Migration Paths

### Scenario 1: Basic Context Storage

**Before (v0.1.0):**
```rust
use tokitai_context::{ContextRoot, FileContextService, FileContextConfig};
use tokitai_context::layers::ContentType;

let root = ContextRoot::new("./.context")?;
let config = FileContextConfig::default();
let mut service = FileContextService::new(&root, config)?;

let hash = service.store(
    "session-1", 
    b"Hello, World!", 
    ContentType::ShortTerm
)?;

let content = service.retrieve("session-1", &hash)?;
```

**After (v0.2.0):**
```rust
use tokitai_context::facade::{Context, Layer};

let mut ctx = Context::open("./.context")?;

let hash = ctx.store("session-1", b"Hello, World!", Layer::ShortTerm)?;

let item = ctx.retrieve("session-1", &hash)?;
let content = &item.content;
```

**Benefits:**
- 40% less boilerplate code
- Automatic WAL logging
- Built-in recovery support

---

### Scenario 2: Parallel Context Management

**Before (v0.1.0):**
```rust
use tokitai_context::{ParallelContextManager, ParallelContextManagerConfig};

let config = ParallelContextManagerConfig {
    context_root: std::path::PathBuf::from("./.context"),
    enable_caching: true,
    ..Default::default()
};

let mut manager = ParallelContextManager::new(config)?;
let branch = manager.create_branch("feature", "main")?;
manager.checkout(&branch.branch_id)?;
```

**After (v0.2.0):**
```rust
use tokitai_context::{ParallelContextManager, ParallelContextManagerConfig};

// Option 1: Same API (still supported)
let config = ParallelContextManagerConfig {
    context_root: std::path::PathBuf::from("./.context"),
    enable_caching: true,
    ..Default::default()
};
let mut manager = ParallelContextManager::new(config)?;

// Option 2: Use Facade API (recommended for simple cases)
use tokitai_context::facade::Context;
let mut ctx = Context::open("./.context")?;
// Branch operations via ctx.parallel_*() methods (if needed)
```

**Note:** The parallel context management API remains unchanged for advanced users.

---

### Scenario 3: Semantic Search

**Before (v0.1.0):**
```rust
use tokitai_context::semantic_index::{SemanticIndex, SemanticIndexConfig};

let config = SemanticIndexConfig::default();
let mut index = SemanticIndex::new("./.context/index", config)?;

index.add("Rust programming", std::path::Path::new("/doc/rust.txt"))?;

let results = index.search("systems programming")?;
for result in results {
    println!("{} - score: {}", result.path.display(), result.score);
}
```

**After (v0.2.0):**
```rust
use tokitai_context::facade::{Context, Layer};

let mut ctx = Context::open("./.context")?;

// Store content (automatically indexed if semantic search enabled)
ctx.store("session-1", b"Rust programming", Layer::LongTerm)?;

// Search across session content
let hits = ctx.search("session-1", "systems programming")?;
for hit in hits {
    println!("{} - score: {}", hit.hash, hit.score);
}
```

**Benefits:**
- Semantic search integrated into main API
- No manual index management required

---

### Scenario 4: Knowledge Management

**Before (v0.1.0):**
```rust
use tokitai_context::{KnowledgeIndex, KnowledgeWatcher};

let index = KnowledgeIndex::from_directory("./knowledge")?;
let watcher = KnowledgeWatcher::new("./knowledge", index.clone())?;

let recommendations = index.recommend("async Rust", 10);
```

**After (v0.2.0):**
```rust
use tokitai_context::KnowledgeManager;

let knowledge = KnowledgeManager::new(
    Some("./knowledge"),
    true,   // auto_recommend
    0.7,    // recommend_threshold
    10,     // recommend_limit
)?;

let recommendations = knowledge.recommend("async Rust");
// Returns Vec<&KnowledgeNode>
```

**Benefits:**
- Unified manager for index + watcher
- Simpler configuration

---

### Scenario 5: Crash Recovery (NEW)

**Before (v0.1.0):**
```rust
// No built-in recovery mechanism
// Manual integrity checks required
```

**After (v0.2.0):**
```rust
use tokitai_context::facade::Context;

let mut ctx = Context::open("./.context")?;

// Check and recover automatically
let report = ctx.recover()?;

if !report.is_clean() {
    println!("Recovered {} operations", report.recovered_operations);
    println!("{} incomplete operations rolled back", 
             report.incomplete_operations);
}

// Continue with normal operations
let hash = ctx.store("session-1", b"Safe!", Layer::ShortTerm)?;
```

**Benefits:**
- Automatic crash recovery
- WAL ensures data integrity
- Detailed recovery reports

---

## Feature Flag Migration

**Old Cargo.toml:**
```toml
[dependencies]
tokitai-context = { version = "0.1", features = ["ai", "benchmarks"] }
```

**New Cargo.toml:**
```toml
[dependencies]
tokitai-context = { version = "0.2", features = ["ai", "benchmarks"] }
# Or use the 'full' feature for everything:
# tokitai-context = { version = "0.2", features = ["full"] }
```

### Available Features

| Feature | Description | Default |
|---------|-------------|---------|
| `wal` | Write-Ahead Log for crash recovery | ✅ Yes |
| `ai` | AI-powered conflict resolution | ❌ No |
| `benchmarks` | Performance benchmarking suite | ❌ No |
| `core` | Minimal dependencies (storage only) | ❌ No |
| `full` | All features | ❌ No |

---

## API Reference Mapping

### Old → New API Mapping

| Old API | New API | Notes |
|---------|---------|-------|
| `ContextRoot::new()` | `Context::open()` | Facade API recommended |
| `FileContextService::store()` | `Context::store()` | Same semantics |
| `FileContextService::retrieve()` | `Context::retrieve()` | Returns `ContextItem` |
| `FileContextService::delete()` | `Context::delete()` | Same semantics |
| `SemanticIndex::search()` | `Context::search()` | Returns `SearchHit` |
| N/A | `Context::recover()` | NEW: Crash recovery |
| N/A | `Context::stats()` | NEW: Statistics |
| `KnowledgeIndex::recommend()` | `KnowledgeManager::recommend()` | Unified manager |

### Return Type Changes

**Old:**
```rust
service.retrieve(...) -> Result<Vec<u8>>
index.search(...) -> Result<Vec<SearchResult>>
```

**New:**
```rust
ctx.retrieve(...) -> Result<ContextItem>  // Contains hash, content, summary
ctx.search(...) -> Result<Vec<SearchHit>> // Contains hash, score, summary
```

---

## Step-by-Step Migration Process

### Step 1: Update Dependencies

```toml
# Cargo.toml
[dependencies]
tokitai-context = { version = "0.2", features = ["wal"] }  # WAL enabled by default
```

### Step 2: Update Imports

```rust
// Old
use tokitai_context::{ContextRoot, FileContextService};

// New
use tokitai_context::facade::{Context, Layer};
```

### Step 3: Replace Initialization

```rust
// Old
let root = ContextRoot::new("./.context")?;
let service = FileContextService::new(&root, Default::default())?;

// New
let mut ctx = Context::open("./.context")?;
```

### Step 4: Update Method Calls

```rust
// Old
let hash = service.store("session", content, ContentType::ShortTerm)?;

// New
let hash = ctx.store("session", content, Layer::ShortTerm)?;
```

### Step 5: Add Recovery (Optional but Recommended)

```rust
let mut ctx = Context::open("./.context")?;

// Add recovery check
let report = ctx.recover()?;
if !report.is_clean() {
    tracing::warn!("Recovered {} operations", report.recovered_operations);
}
```

### Step 6: Run Tests

```bash
cargo test
cargo clippy --all-targets --all-features
```

---

## Troubleshooting

### Issue: "Module not found" errors

**Solution:** Update imports to use new module paths:

```rust
// Old (no longer works)
use tokitai_context::semantic_index::SemanticIndex;

// New
use tokitai_context::core::SemanticIndex;

// Or use facade:
use tokitai_context::facade::Context;
```

### Issue: Type mismatch with `retrieve()`

**Solution:** The new API returns `ContextItem` instead of `Vec<u8>`:

```rust
// Old
let content: Vec<u8> = service.retrieve(...)?;

// New
let item: ContextItem = ctx.retrieve(...)?;
let content: &Vec<u8> = &item.content;
```

### Issue: Missing `ContentType` enum

**Solution:** Use `Layer` enum instead:

```rust
// Old
use tokitai_context::layers::ContentType;
ContentType::Transient

// New
use tokitai_context::facade::Layer;
Layer::Transient
```

### Issue: WAL performance concerns

**Solution:** WAL is optimized for minimal overhead (~2-3ms per operation). If you need maximum performance:

```toml
# Disable WAL in Cargo.toml
tokitai-context = { version = "0.2", default-features = false, features = ["core"] }
```

---

## Backward Compatibility

The old API paths are still available via re-exports for backward compatibility:

```rust
// Still works in v0.2.0
use tokitai_context::ContextRoot;
use tokitai_context::FileContextService;
use tokitai_context::ParallelContextManager;
```

However, new features (WAL, recovery, facade API) are only available through the new interfaces.

---

## Getting Help

- **Documentation**: See `doc/` directory for detailed guides
- **Examples**: Check `examples/` directory (if available)
- **Issues**: Report migration issues on GitHub
- **Discord**: Join the Tokitai community for real-time help

---

## Summary

| Aspect | v0.1.0 | v0.2.0 |
|--------|--------|--------|
| API Complexity | Low-level | High-level facade |
| Crash Recovery | Manual | Automatic (WAL) |
| Module Structure | Flat | Layered (core/parallel/optimization/ai) |
| Error Messages | Basic | Enhanced |
| Tracing | Partial | Complete |
| Lines of Code | More | Less (40% reduction) |

**Recommendation:** All new projects should use the Facade API. Existing projects should migrate when convenient, but the old API remains supported.
