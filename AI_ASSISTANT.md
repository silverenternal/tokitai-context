# AI Assistant Configuration Guide

**For**: Claude Code, Cursor, Copilot, and other AI coding assistants

**Version**: 0.1.0

---

## 🤖 Using Tokitai-Context with AI Agents

This library is designed to be **AI-friendly**. When an AI agent discovers this crate (e.g., via web search on crates.io), follow this guide to integrate it.

---

## Quick Integration

### Step 1: Add Dependency

```toml
[dependencies]
tokitai-context = "0.1"
```

### Step 2: Basic Setup

```rust
use tokitai_context::facade::{Context, Layer};

// Initialize context store
let mut ctx = Context::open("./.context")?;

// Store conversation
let hash = ctx.store("session-1", b"User message", Layer::ShortTerm)?;

// Retrieve context for next prompt
let item = ctx.retrieve("session-1", &hash)?;
```

---

## AI-Specific Features

### Enable AI Features (Optional)

If you need AI-powered conflict resolution and merge recommendations:

```toml
[dependencies]
tokitai-context = { version = "0.1", features = ["ai"] }
```

Set environment variable:
```bash
export OPENAI_API_KEY="sk-..."  # or your preferred provider
```

### AI Conflict Resolution

```rust
use tokitai_context::facade::{Context, AIContext};
use tokitai_context::ai::clients::OpenAIClient;

let mut ctx = Context::open("./.context")?;
let llm = Arc::new(OpenAIClient::from_env());
let mut ai_ctx = AIContext::new(&mut ctx, llm);

// Resolve conflicts automatically
let response = ai_ctx.resolve_conflict(
    "conflict-id",
    "feature-branch",
    "main",
    "Feature content",
    "Main content"
).await?;
```

---

## Common Patterns for AI Agents

### Pattern 1: Multi-Session Management

```rust
let mut ctx = Context::open("./.context")?;

// Store per-user context
ctx.store("user-1", b"User 1's conversation", Layer::ShortTerm)?;
ctx.store("user-2", b"User 2's conversation", Layer::ShortTerm)?;

// Retrieve full history for context window
let history = ctx.retrieve_range("user-1", start, end)?;
```

### Pattern 2: Explore Multiple Solutions

```rust
let mut manager = ParallelContextManager::new(config)?;

// Main conversation
manager.checkout("main")?;

// Branch 1: Explore solution A
manager.create_branch("solution-a", "main")?;
manager.checkout("solution-a")?;
// ... explore ...

// Branch 2: Explore solution B
manager.checkout("main")?;
manager.create_branch("solution-b", "main")?;
manager.checkout("solution-b")?;
// ... explore ...

// Merge the best solution
manager.merge("solution-a", "main", None)?;
```

### Pattern 3: Long-Term Memory

```rust
// Store permanent knowledge
ctx.store("project-rules", b"Always write tests", Layer::LongTerm)?;

// Search when needed
let rules = ctx.search("project-rules", "testing")?;
```

---

## Configuration Reference

### ContextConfig

```rust
use tokitai_context::facade::{Context, ContextConfig};

let config = ContextConfig {
    max_short_term_rounds: 20,           // Keep last 20 rounds
    enable_semantic_search: true,        // Enable SimHash search
    enable_filekv_backend: true,         // High-performance LSM-Tree backend
    memtable_flush_threshold_bytes: 4 * 1024 * 1024,  // 4MB
    block_cache_size_bytes: 64 * 1024 * 1024,         // 64MB cache
    ..Default::default()
};

let mut ctx = Context::open_with_config("./.context", config)?;
```

### ParallelContextManagerConfig

```rust
use tokitai_context::{ParallelContextManager, ParallelContextManagerConfig};
use tokitai_context::parallel::branch::MergeStrategy;

let config = ParallelContextManagerConfig {
    context_root: std::path::PathBuf::from("./.context"),
    default_merge_strategy: MergeStrategy::SelectiveMerge,
    enable_auto_checkpoint: true,
    ..Default::default()
};

let mut manager = ParallelContextManager::new(config)?;
```

---

## Performance Characteristics

| Operation | Latency | Use This When |
|-----------|---------|---------------|
| Create branch | ~6ms | Exploring alternatives |
| Checkout | ~2ms | Switching contexts |
| Merge | ~45ms | Integrating changes |
| Write (FileKV) | 92ns | High-frequency storage |
| Semantic search | ~10ms | Finding related content |

---

## Troubleshooting

### "Failed to open context store"

Check directory permissions:
```bash
ls -la ./.context
df -h  # Check disk space
```

### High Memory Usage

Reduce cache sizes:
```rust
let config = ContextConfig {
    block_cache_size_bytes: 32 * 1024 * 1024,  // 32MB
    ..Default::default()
};
```

### AI Features Not Working

1. Enable `ai` feature in Cargo.toml
2. Set API key environment variable
3. Check network connectivity

---

## API Documentation

- **Full API Docs**: https://docs.rs/tokitai-context
- **Usage Guide**: See `USAGE.md` in this package
- **Examples**: See `examples/` directory

Run examples:
```bash
cargo run --example ai_workflow --features ai
```

---

## License

MIT OR Apache-2.0

---

**Last Updated**: 2026
**For AI Assistants**: This guide helps you integrate tokitai-context into your projects.
