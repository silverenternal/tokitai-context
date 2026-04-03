# P3-004: Distributed Coordination with etcd

## Overview

This module provides distributed coordination primitives for multi-node tokitai deployments, enabling:
- **Distributed Locking**: Mutual exclusion across nodes using etcd leases
- **Leader Election**: Automatic leader election with failover support
- **Coordination Manager**: Unified management for coordination primitives

## Features

### Distributed Lock
- Lease-based locking with automatic expiration
- Automatic cleanup on node failure
- Try-acquire with timeout support
- Thread-safe async API

### Leader Election
- Lease-based leadership with automatic renewal
- Instant failover notification via etcd watch
- Graceful resignation support
- Leader state tracking

### Coordination Manager
- Shared etcd connection for efficiency
- Unified metrics collection
- Factory for locks and elections
- Prometheus metrics export

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    CoordinationManager                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ Distributed │  │ Distributed │  │    CoordinationStats    │ │
│  │    Lock 1   │  │    Lock 2   │  │  - lock acquisitions    │ │
│  └─────────────┘  └─────────────┘  │  - leader elections     │ │
│  ┌─────────────┐  ┌─────────────┐  │  - connection errors    │ │
│  │  Leader     │  │  Leader     │  │  - prometheus export    │ │
│  │ Election 1  │  │ Election 2  │  └─────────────────────────┘ │
│  └─────────────┘  └─────────────┘                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │   etcd Client   │
                    │  (shared conn)  │
                    └─────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  etcd Cluster   │
                    │  (2379,2380...) │
                    └─────────────────┘
```

## Usage

### Basic Setup

```rust
use tokitai_context::distributed_coordination::{
    CoordinationConfig, CoordinationManager, DistributedLock, LeaderElection
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure etcd connection
    let config = CoordinationConfig::new(vec!["http://localhost:2379"])
        .with_lease_ttl(30)
        .with_key_prefix("/myapp");

    // Create coordination manager
    let mut manager = CoordinationManager::new(config);
    manager.connect().await?;

    Ok(())
}
```

### Distributed Lock

```rust
// Create a lock for a shared resource
let mut lock = manager.create_lock("database-migration".to_string());

// Acquire the lock
lock.acquire().await?;

// Critical section - only one node can execute here
println!("Running migration...");
perform_migration().await?;

// Release the lock
lock.release().await?;
```

### Try Acquire with Timeout

```rust
let mut lock = manager.create_lock("shared-resource".to_string());

// Try to acquire with 5 second timeout
match lock.try_acquire(Duration::from_secs(5)).await {
    Ok(true) => {
        println!("Lock acquired, executing critical section");
        // Critical section
        lock.release().await?;
    }
    Ok(false) => {
        println!("Lock not available, trying alternative path");
    }
    Err(e) => {
        eprintln!("Lock acquisition failed: {}", e);
    }
}
```

### Leader Election

```rust
// Create leader election
let mut election = manager.create_election("cluster-leader".to_string());

// Start participating in election
election.start().await?;

// Check if we are the leader
if election.is_leader().await? {
    println!("We are the leader!");
    
    // Run leader-specific tasks
    run_leader_tasks().await?;
}

// Later, optionally resign
election.resign().await?;
```

### Leader with Callback

```rust
let mut election = manager.create_election("worker-coordinator".to_string());
election.start().await?;

// Periodically check leadership
let mut interval = tokio::time::interval(Duration::from_secs(10));
loop {
    interval.tick().await;
    
    if election.is_leader().await.unwrap_or(false) {
        // Perform leader duties
        coordinate_workers().await?;
    }
}
```

### Prometheus Metrics

```rust
// Export coordination metrics
let metrics = manager.to_prometheus();
println!("{}", metrics);

// Example output:
// tokitai_coordination_lock_acquisitions_total 42
// tokitai_coordination_lock_releases_total 40
// tokitai_coordination_leader_elections_total 5
// tokitai_coordination_leader_failovers_total 2
// tokitai_coordination_connection_errors_total 0
// tokitai_coordination_is_leader 1
```

## Configuration

### CoordinationConfig Options

```rust
let config = CoordinationConfig {
    // etcd endpoints
    endpoints: vec![
        "http://etcd-1:2379".to_string(),
        "http://etcd-2:2379".to_string(),
        "http://etcd-3:2379".to_string(),
    ],
    
    // Connection timeout
    connect_timeout: Duration::from_secs(5),
    
    // Lease TTL in seconds (how long lock/election lasts without renewal)
    lease_ttl: 30,
    
    // Keepalive interval (should be < lease_ttl / 3)
    keepalive_interval: Duration::from_secs(3),
    
    // Optional authentication
    username: Some("etcd_user".to_string()),
    password: Some("secret".to_string()),
    
    // Key prefix for all coordination keys
    key_prefix: "/tokitai".to_string(),
};
```

## Key Design Decisions

### Lease-Based Locking

We use etcd leases instead of simple key-value pairs because:
1. **Automatic Expiration**: If a node crashes, the lease expires and the lock is automatically released
2. **No Stale Locks**: Eliminates the need for manual lock cleanup
3. **Efficient Keepalive**: Single keepalive for all locks sharing a lease

### Leader Election Pattern

The election uses a compare-and-swap transaction:
```
TXN:
  IF leader_key.version == 0:
    PUT leader_key = <our_id> WITH LEASE
  ELSE:
    GET current_leader
```

This ensures exactly one leader at any time.

### Watch-Based Failover

Followers watch the leader key for changes:
- If leader key is deleted → election opportunity
- If leader key changes → new leader elected
- Instant notification without polling

## Error Handling

### CoordinationError Types

```rust
pub enum CoordinationError {
    Etcd(EtcdError),           // etcd client errors
    ConnectionFailed(String),  // etcd connection failures
    LockAcquisitionFailed(String),
    LockReleaseFailed(String),
    LeaderElectionFailed(String),
    Timeout(String),
    NotLeader,                 // Operation requires leadership
    NotConnected,
    InvalidConfig(String),
}
```

### Retry Strategy

For transient errors, implement retry logic:

```rust
use tokio::time::{sleep, Duration};

async fn acquire_with_retry(lock: &mut DistributedLock, max_retries: u32) -> CoordinationResult<()> {
    for attempt in 1..=max_retries {
        match lock.acquire().await {
            Ok(()) => return Ok(()),
            Err(CoordinationError::ConnectionFailed(_)) if attempt < max_retries => {
                sleep(Duration::from_millis(100 * attempt)).await;
            }
            Err(e) => return Err(e),
        }
    }
    Err(CoordinationError::Timeout("Max retries exceeded".to_string()))
}
```

## Testing

### Unit Tests

Run unit tests (no etcd required):

```bash
cargo test --lib distributed_coordination::tests
```

### Integration Tests

For full integration tests, start etcd:

```bash
# Using docker
docker run -d --name etcd -p 2379:2379 quay.io/coreos/etcd:latest etcd

# Run tests
cargo test --features distributed distributed_coordination::integration_tests
```

## Performance Considerations

### Connection Pooling

The `CoordinationManager` shares a single etcd connection across all locks and elections, reducing:
- TCP connection overhead
- TLS handshake latency
- Memory footprint

### Lease TTL Tuning

| TTL | Pros | Cons |
|-----|------|------|
| Short (5-10s) | Fast failover | More keepalive traffic |
| Medium (30s) | Balanced | Moderate failover time |
| Long (60s+) | Less traffic | Slow failover |

Recommended: 30s TTL with 10s keepalive interval.

### Batch Operations

For multiple locks, consider hierarchical locking:

```rust
// Instead of many fine-grained locks:
let mut lock1 = manager.create_lock("resource-1".to_string());
let mut lock2 = manager.create_lock("resource-2".to_string());
// ...

// Use a coarse-grained lock:
let mut lock = manager.create_lock("resource-group".to_string());
```

## Monitoring

### Key Metrics

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `lock_acquisitions_total` | Rate of lock acquisitions | Sudden drop |
| `lock_releases_total` | Rate of lock releases | Should match acquisitions |
| `leader_elections_total` | Number of leader elections | > 1 per minute |
| `leader_failovers_total` | Unexpected leader changes | > 0 |
| `connection_errors_total` | etcd connection failures | > 0 |
| `is_leader` | Current leader status (0/1) | Frequent flipping |

### Grafana Dashboard

```promql
# Lock acquisition rate
rate(tokitai_coordination_lock_acquisitions_total[5m])

# Leader election frequency
rate(tokitai_coordination_leader_elections_total[5m])

# Connection error rate
rate(tokitai_coordination_connection_errors_total[1m])
```

## Troubleshooting

### Lock Not Released

**Symptoms**: Lock held indefinitely, other nodes can't acquire

**Causes**:
1. Node crashed without graceful shutdown
2. Network partition isolated the lock holder
3. Lease keepalive failing

**Solutions**:
- Wait for lease TTL to expire (automatic)
- Check etcd connectivity
- Verify lease keepalive is running

### Leader Flapping

**Symptoms**: Frequent leader elections, `leader_failovers_total` increasing

**Causes**:
1. Network instability
2. etcd cluster performance issues
3. TTL too short for network conditions

**Solutions**:
- Increase lease TTL
- Improve network reliability
- Check etcd cluster health

### Connection Failures

**Symptoms**: `connection_errors_total` increasing, operations failing

**Causes**:
1. etcd cluster unavailable
2. Network partition
3. Authentication failure

**Solutions**:
- Verify etcd cluster health: `etcdctl endpoint health`
- Check network connectivity
- Verify credentials

## Security Considerations

### Authentication

Always use authentication in production:

```rust
let config = CoordinationConfig::new(vec!["https://etcd:2379"])
    .with_auth("tokitai", "strong-password")
    .with_key_prefix("/tokitai/prod");
```

### TLS

Use TLS for etcd connections:

```rust
let options = ConnectOptions::new()
    .with_secure(true)
    .with_ca_cert("/etc/etcd/ca.crt")
    .with_cert("/etc/etcd/client.crt", "/etc/etcd/client.key");
```

### Key Prefix Isolation

Use different prefixes for environments:

```rust
let dev_config = config.clone().with_key_prefix("/tokitai/dev");
let prod_config = config.clone().with_key_prefix("/tokitai/prod");
```

## Future Enhancements

- [ ] Support for etcd v3.5+ features
- [ ] Multi-region coordination
- [ ] Quorum-based locking
- [ ] Distributed semaphores
- [ ] Rate limiting primitives
- [ ] Integration with service mesh

## References

- [etcd Documentation](https://etcd.io/docs/)
- [etcd Concurrency Patterns](https://etcd.io/docs/v3.5/learning/concurrency/)
- [Distributed Locks with etcd](https://etcd.io/docs/v3.5/learning/api-concurrency/)
- [etcd-client Rust Crate](https://docs.rs/etcd-client/)
