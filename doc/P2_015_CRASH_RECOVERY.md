# P2-015: Crash Recovery Test Framework

## Overview

The Crash Recovery Test Framework provides comprehensive fault injection capabilities for testing data consistency and durability guarantees under various failure conditions.

## Components

### 1. Fault Injector (`fault_injection.rs`)

Core fault injection engine with:
- Configurable failure rates (0.0 - 1.0)
- Multiple fault types
- Deterministic random for reproducible tests
- Thread-safe operation

### 2. Integration Tests (`integration_tests.rs`)

Pre-built crash scenarios and test harness for:
- WAL crash recovery
- MemTable flush failures
- Segment write failures
- Index update failures
- Compaction failures
- Disk full simulation

## Fault Types

| Fault Type | Description | Test Scenario |
|------------|-------------|---------------|
| `WalWriteFailure` | WAL write operation fails | WAL corruption during write |
| `MemTableFlushFailure` | MemTable flush fails | Data loss during flush |
| `SegmentWriteFailure` | Segment file write fails | Incomplete segment data |
| `IndexUpdateFailure` | Index update fails | Index points to invalid data |
| `BloomFilterWriteFailure` | Bloom filter write fails | Missing bloom filters |
| `CompactionFailure` | Compaction operation fails | Incomplete compaction |
| `CheckpointFailure` | Checkpoint operation fails | Corrupted checkpoint |
| `DiskFull` | Disk full simulation | No space for writes |
| `RandomCrash` | Random crash simulation | Unexpected termination |

## Usage

### Basic Fault Injection

```rust
use tokitai_context::crash_recovery::{FaultInjector, FaultType};

// Create fault injector
let injector = FaultInjector::new();

// Enable fault with 30% failure rate
injector.enable_fault(FaultType::WalWriteFailure, 0.3);

// Execute operation with potential fault
let result = injector.execute(&FaultType::WalWriteFailure, || {
    kv.put("key", b"value")
})?;

// Check statistics
let stats = injector.stats();
println!("Injected {} faults", stats.injected_faults);
```

### Deterministic Testing

```rust
// Use seed for reproducible tests
let injector = FaultInjector::with_seed(42);
injector.enable_fault(FaultType::WalWriteFailure, 0.5);

// Run test - same sequence of faults every time
for i in 0..100 {
    if injector.should_fail(&FaultType::WalWriteFailure) {
        // Handle fault
    }
}
```

### Max Failures Limit

```rust
use tokitai_context::crash_recovery::FaultConfig;

let config = FaultConfig {
    failure_rate: 1.0, // 100% failure
    max_failures: Some(3), // Only fail 3 times
    enabled: true,
    seed: None,
};

injector.configure(FaultType::MemTableFlushFailure, config);
```

### Crash Scenario Testing

```rust
use tokitai_context::crash_recovery::{
    CrashScenario, CrashRecoveryHarness, CrashRecoveryResult,
};

// Create test harness
let harness = CrashRecoveryHarness::new();

// Define scenario
let scenario = CrashScenario::new(
    "wal_crash_test",
    "WAL fails during write operations",
    FaultType::WalWriteFailure,
    0.3, // 30% failure rate
    1000, // 1000 operations
);

// Run scenario
let result = harness.run_scenario(&scenario);

// Check results
assert!(result.data_consistent, "Data should be consistent");
println!("Recovery time: {}ms", result.recovery_time_ms);
```

### Running All Standard Scenarios

```rust
let harness = CrashRecoveryHarness::new();
let results = harness.run_all_scenarios();

// Print summary
println!("{}", CrashRecoveryHarness::summarize_results(&results));

// Check individual results
for result in results {
    if !result.success {
        eprintln!("Failed: {}", result.scenario_name);
        eprintln!("Error: {:?}", result.error_message);
    }
}
```

## Standard Scenarios

The framework includes 6 pre-defined crash scenarios:

| Scenario | Fault Type | Failure Rate | Operations |
|----------|-----------|--------------|------------|
| `wal_crash_during_write` | WalWriteFailure | 30% | 1000 |
| `memtable_flush_crash` | MemTableFlushFailure | 20% | 500 |
| `segment_write_crash` | SegmentWriteFailure | 25% | 200 |
| `index_update_crash` | IndexUpdateFailure | 20% | 300 |
| `compaction_crash` | CompactionFailure | 15% | 100 |
| `disk_full_scenario` | DiskFull | 10% | 500 |

## Test Results

### Unit Tests (10 tests)

```
test crash_recovery::fault_injection::tests::test_crash_recovery_result ... ok
test crash_recovery::fault_injection::tests::test_crash_scenario_creation ... ok
test crash_recovery::fault_injection::tests::test_fault_injector_disabled_by_default ... ok
test crash_recovery::fault_injection::tests::test_fault_injector_execute ... ok
test crash_recovery::fault_injection::tests::test_fault_injector_enable_disable ... ok
test crash_recovery::fault_injection::tests::test_fault_injector_reset ... ok
test crash_recovery::fault_injection::tests::test_standard_scenarios ... ok
test crash_recovery::fault_injection::tests::test_fault_injector_max_failures ... ok
test crash_recovery::fault_injection::tests::test_fault_injector_stats ... ok
test crash_recovery::fault_injection::tests::test_fault_injector_failure_rate ... ok
```

### Integration Tests (6 tests)

```
test crash_recovery::integration_tests::tests::test_harness_creation ... ok
test crash_recovery::integration_tests::tests::test_recovery_after_fault ... ok
test crash_recovery::integration_tests::tests::test_segment_write_crash_scenario ... ok
test crash_recovery::integration_tests::tests::test_memtable_flush_crash_scenario ... ok
test crash_recovery::integration_tests::tests::test_wal_crash_scenario ... ok
test crash_recovery::integration_tests::tests::test_multiple_scenarios ... ok
```

**Total: 16/16 tests pass**

## API Reference

### FaultInjector

```rust
pub struct FaultInjector {
    // Thread-safe fault injection
}

impl FaultInjector {
    pub fn new() -> Self;
    pub fn with_seed(seed: u64) -> Self;
    
    pub fn enable(&self);
    pub fn disable(&self);
    pub fn is_enabled(&self) -> bool;
    
    pub fn enable_fault(&self, fault_type: FaultType, failure_rate: f64);
    pub fn disable_fault(&self, fault_type: &FaultType);
    pub fn configure(&self, fault_type: FaultType, config: FaultConfig);
    pub fn clear(&self);
    pub fn reset(&self);
    
    pub fn should_fail(&self, fault_type: &FaultType) -> bool;
    pub fn execute<T, F, E>(&self, fault_type: &FaultType, operation: F) -> Result<T, E>;
    pub fn stats(&self) -> FaultStats;
}
```

### CrashScenario

```rust
pub struct CrashScenario {
    pub name: String,
    pub description: String,
    pub fault_type: FaultType,
    pub failure_rate: f64,
    pub operations_count: usize,
    pub expected_recovery_time_ms: u64,
}

impl CrashScenario {
    pub fn new(...) -> Self;
    pub fn standard_scenarios() -> Vec<CrashScenario>;
}
```

### CrashRecoveryResult

```rust
pub struct CrashRecoveryResult {
    pub scenario_name: String,
    pub success: bool,
    pub operations_attempted: usize,
    pub operations_succeeded: usize,
    pub faults_injected: usize,
    pub recovery_time_ms: u64,
    pub data_consistent: bool,
    pub error_message: Option<String>,
}
```

### FaultStats

```rust
pub struct FaultStats {
    pub total_attempts: usize,
    pub injected_faults: usize,
    pub successful_ops: usize,
    pub faults_by_type: HashMap<String, usize>,
}
```

## Example: Testing WAL Recovery

```rust
#[test]
fn test_wal_recovery() {
    let harness = CrashRecoveryHarness::new();
    
    // Simulate WAL failures during writes
    let scenario = CrashScenario::new(
        "wal_recovery_test",
        "Test WAL recovery after crashes",
        FaultType::WalWriteFailure,
        0.5,
        100,
    );
    
    let result = harness.run_scenario(&scenario);
    
    // Verify data consistency after recovery
    assert!(result.data_consistent, 
        "Data should be consistent after WAL recovery");
    
    // Verify recovery time is acceptable
    assert!(result.recovery_time_ms < 5000,
        "Recovery should complete within 5 seconds");
}
```

## Example: Custom Crash Scenario

```rust
#[test]
fn test_custom_crash_scenario() {
    let injector = FaultInjector::with_seed(12345);
    
    // Configure custom fault pattern
    let config = FaultConfig {
        failure_rate: 0.4,
        max_failures: Some(10),
        enabled: true,
        seed: Some(12345),
    };
    
    injector.configure(FaultType::CompactionFailure, config);
    injector.enable();
    
    // Run compaction with fault injection
    let mut failures = 0;
    for i in 0..100 {
        if injector.should_fail(&FaultType::CompactionFailure) {
            failures += 1;
            // Simulate recovery
        }
    }
    
    // Should have exactly 10 failures (max_failures limit)
    assert_eq!(failures, 10);
}
```

## Integration with CI/CD

Add crash recovery tests to CI pipeline:

```yaml
# .github/workflows/test.yml
- name: Run crash recovery tests
  run: cargo test --lib crash_recovery -- --test-threads=1

- name: Check crash recovery coverage
  run: cargo tarpaulin --out Xml --files crash_recovery
```

## Best Practices

### 1. Use Deterministic Seeds

For reproducible tests, always use `with_seed()`:

```rust
let injector = FaultInjector::with_seed(42);
```

### 2. Test Multiple Failure Rates

Test different failure rates to cover various scenarios:

```rust
for rate in [0.1, 0.3, 0.5, 0.7, 0.9] {
    injector.enable_fault(FaultType::WalWriteFailure, rate);
    // Run test
}
```

### 3. Verify Data Consistency

Always verify data consistency after fault injection:

```rust
assert!(result.data_consistent, "Data must be consistent after recovery");
```

### 4. Test Recovery Time

Ensure recovery completes within acceptable time:

```rust
assert!(result.recovery_time_ms < max_allowed_ms);
```

### 5. Clean Up Between Tests

Reset fault injector between tests:

```rust
#[test]
fn test_something() {
    let injector = FaultInjector::new();
    // ... test code ...
    injector.reset();
}
```

## Limitations

- Fault injection is software-based (not kernel-level)
- Some race conditions may not be reproducible
- Disk full simulation is approximate
- Network failures not simulated (single-node only)

## Future Improvements

- [ ] Kernel-level fault injection (eBPF, dtrace)
- [ ] Network partition simulation
- [ ] Multi-node crash testing
- [ ] Performance regression detection
- [ ] Automated crash report generation
- [ ] Chaos engineering integration

## Related Issues

- **P0-005**: Compaction atomicity
- **P1-005**: Test coverage gaps
- **P2-015**: This implementation

## Files Created

- `src/crash_recovery/mod.rs` - Module definition
- `src/crash_recovery/fault_injection.rs` - Fault injection engine
- `src/crash_recovery/integration_tests.rs` - Integration tests
- `doc/P2_015_CRASH_RECOVERY.md` - This documentation

## Verification

```bash
# Build
cargo build --lib

# Clippy (0 warnings)
cargo clippy --lib

# Run tests
cargo test --lib crash_recovery

# Run specific test
cargo test --lib crash_recovery::fault_injection::tests::test_fault_injector_failure_rate
```

## Conclusion

The Crash Recovery Test Framework provides comprehensive fault injection capabilities with:

- ✅ 16 passing tests (10 unit + 6 integration)
- ✅ 9 fault types covered
- ✅ 6 standard crash scenarios
- ✅ Deterministic testing with seeds
- ✅ Thread-safe operation
- ✅ Easy integration with existing tests

The framework ensures data consistency and durability guarantees are maintained under various failure conditions.
