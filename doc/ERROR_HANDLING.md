# Unified Error Handling Strategy

**Issue:** P2-001  
**Status:** In Progress  
**Date:** 2026-04-03

## Overview

This document defines the unified error handling strategy for tokitai-context. The goal is to provide consistent, informative, and recoverable error handling across all modules.

## Error Types

### Core Error Types

The crate defines several module-specific error types:

1. **`FileKVError`** - FileKV storage operations
2. **`ContextError`** - Context management (graph, branch, merge)
3. **`IndexError`** - Sparse index operations
4. **`WalError`** - Write-Ahead Log operations
5. **`CompactionError`** - Compaction operations
6. **`CacheError`** - Cache operations

### Error Type Structure

Each error type follows this pattern:

```rust
#[derive(Debug, Error)]
pub enum ModuleError {
    /// Specific error variant with context
    #[error("Descriptive message: {context}")]
    SpecificError { context: String },
    
    /// I/O errors (automatically converted via From trait)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ModuleError>;
```

## Guidelines

### Public API (Facade, Managers)

- **NEVER** use `anyhow::bail!()` in public APIs
- Return module-specific error types
- Use `?` operator to propagate errors
- Provide context-rich error messages

```rust
// ✅ Good
pub fn get(&self, key: &str) -> Result<Vec<u8>> {
    if let Some(value) = self.cache.get(key) {
        Ok(value)
    } else {
        Err(FileKVError::KeyNotFound(key.to_string()))
    }
}

// ❌ Bad
pub fn get(&self, key: &str) -> anyhow::Result<Vec<u8>> {
    if let Some(value) = self.cache.get(key) {
        Ok(value)
    } else {
        anyhow::bail!("Key not found: {}", key)
    }
}
```

### Internal Implementation

- May use `anyhow` for internal error context during migration
- Convert to module-specific errors at boundaries
- Use `.map_err()` for error conversion (not `.with_context()` which creates anyhow errors)

```rust
// ✅ Good - Direct error mapping
std::fs::create_dir_all(&dir)
    .map_err(|e| ModuleError::Io(e))?;

// ❌ Bad - Creates anyhow::Error
std::fs::create_dir_all(&dir)
    .with_context(|| format!("Failed to create dir: {:?}", dir))?;
```

### Error Conversion

Implement `From<T>` traits for seamless conversion:

```rust
// In error.rs
pub enum ContextError {
    Io(#[from] std::io::Error),
    Serialization(#[from] serde_json::Error),
    Internal(#[from] anyhow::Error), // For wrapping internal anyhow errors
}
```

### Error Categories

All errors implement categorization for recovery strategies:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Recoverable,    // Retry may succeed
    Fatal,          // Manual intervention required
    Temporary,      // Wait and retry may succeed
    Config,         // Fix configuration before retry
}

impl FileKVError {
    pub fn category(&self) -> ErrorCategory {
        match self {
            FileKVError::Io(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
                ErrorCategory::Recoverable
            }
            FileKVError::ChecksumMismatch { .. } => ErrorCategory::Fatal,
            FileKVError::Timeout(_) => ErrorCategory::Temporary,
            // ... etc
        }
    }
    
    pub fn is_recoverable(&self) -> bool {
        self.category() == ErrorCategory::Recoverable
            || self.category() == ErrorCategory::Temporary
    }
}
```

## Migration Status

### Completed Modules

- ✅ `sparse_index.rs` - Converted to `IndexError`
- ✅ `graph.rs` - Converted to `ContextError`
- ✅ `error.rs` - Enhanced with `ContextError` and guidelines

### Remaining Modules

- ⏳ `facade.rs` - Still uses `anyhow::Result`
- ⏳ `parallel_manager.rs` - Still uses `anyhow::Result`
- ⏳ `unified_manager.rs` - Still uses `anyhow::bail!()`
- ⏳ `summarizer.rs` - Still uses `anyhow::bail!()`
- ⏳ Other AI/merge modules - Still use `anyhow::bail!()`

## Best Practices

### 1. Provide Context

Error messages should include:
- What operation failed
- What parameters were involved
- What the expected vs actual state was

```rust
// ✅ Good
Err(IndexError::InvalidIndexMagic(index_path.clone()))

// ❌ Bad
Err(IndexError::InvalidFormat("bad magic".to_string()))
```

### 2. Use Specific Error Variants

```rust
// ✅ Good - Specific variants
pub enum ContextError {
    BranchNotFound(String),
    BranchAlreadyExists(String),
    ParentBranchNotFound(String),
}

// ❌ Bad - Generic string error
pub enum ContextError {
    #[error("{0}")]
    Error(String),
}
```

### 3. Preserve Error Chains

When wrapping errors, preserve the original error:

```rust
// ✅ Good - Preserves original error
#[error("I/O error: {0}")]
Io(#[from] std::io::Error),

// ❌ Bad - Loses original error
#[error("I/O failed")]
Io { path: String },
```

### 4. Enable Recovery

Provide recovery suggestions:

```rust
impl FileKVError {
    pub fn recovery_action(&self) -> Option<RecoveryAction> {
        match self {
            FileKVError::Io(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Some(RecoveryAction {
                    description: "Create missing directory or file".to_string(),
                    is_automatic: true,
                    success_probability: 0.9,
                })
            }
            // ... etc
        }
    }
}
```

## Testing

Error handling should be tested:

```rust
#[test]
fn test_error_category() {
    let io_not_found = FileKVError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    assert_eq!(io_not_found.category(), ErrorCategory::Recoverable);
    assert!(io_not_found.is_recoverable());
}

#[test]
fn test_recovery_action() {
    let checksum_error = FileKVError::ChecksumMismatch {
        expected: 0x12345678,
        actual: 0x87654321,
    };
    let action = checksum_error.recovery_action();
    assert!(action.is_some());
    assert!(!action.unwrap().is_automatic); // Requires manual intervention
}
```

## Future Work

1. **Complete Migration**: Convert remaining modules to use custom error types
2. **Error Codes**: Add numeric error codes for programmatic handling
3. **Structured Logging**: Integrate with tracing for structured error logging
4. **Metrics**: Add error rate monitoring and alerting
5. **Documentation**: Generate user-facing error documentation

## References

- [Rust Error Handling Best Practices](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [thiserror crate documentation](https://docs.rs/thiserror)
- [anyhow crate documentation](https://docs.rs/anyhow)
