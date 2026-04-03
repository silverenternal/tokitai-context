# P3-007: Query Optimizer

## Overview

The Query Optimizer module provides sophisticated cost-based query optimization and execution planning for the tokitai-context storage engine. It enables efficient query processing through intelligent plan generation, cost estimation, and execution strategy selection.

## Features

### Query Representation
- **Query Builder**: Fluent API for constructing queries programmatically
- **Query Operations**: Scan, IndexScan, RangeScan, Filter, Project, Limit, Sort, Aggregate, Join, Union, Distinct
- **Query Predicates**: Eq, Ne, Lt, Le, Gt, Ge, In, Like, IsNull, IsNotNull, And, Or, Not
- **Value Types**: Null, Bool, Int, Float, String, Bytes, Array

### Query Planning
- **Logical Plan**: Abstract representation of query operations
- **Physical Plan**: Executable plan with cost estimates and statistics
- **Plan Nodes**: Scan, IndexScan, RangeScan, Filter, Project, Limit, Sort, Aggregate, HashJoin, NestedLoopJoin, MergeJoin, Union, Distinct

### Cost-Based Optimization
- **Cost Model**: Configurable cost parameters for I/O, CPU, and memory operations
- **Statistics**: Table statistics, column statistics, index statistics, histograms
- **Selectivity Estimation**: Predicate selectivity for row count estimation
- **Join Ordering**: Optimal join sequence selection

### Optimization Rules
- **Predicate Pushdown**: Move filters closer to data source
- **Projection Pushdown**: Reduce data early by selecting columns
- **Join Reordering**: Optimize join order for minimal intermediate results
- **Index Selection**: Choose optimal index for access

### Execution Strategies
- **Sort Algorithms**: QuickSort, ExternalMergeSort, TopKHeap
- **Join Algorithms**: HashJoin, NestedLoopJoin, MergeJoin
- **Distinct Methods**: Hash, Sort, BloomFilter
- **Aggregation**: Group-by with multiple aggregate functions

### Query Execution
- **Executor**: Async execution engine with statistics tracking
- **Result Handling**: Structured query results with metadata
- **Performance Metrics**: Execution time, rows affected, cache hits

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Query Optimizer                         │
├─────────────────────────────────────────────────────────────┤
│  Query Builder → Logical Plan → Optimizer → Physical Plan   │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Cost Model   │  │ Statistics   │  │ Plan Cache   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├─────────────────────────────────────────────────────────────┤
│                      Query Executor                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Scan Nodes   │  │ Join Nodes   │  │ Sort/Agg     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Usage

### Basic Query

```rust
use tokitai_context::query_optimizer::{Query, QueryValue, QueryOptimizer};

// Create a query
let query = Query::scan("users")
    .filter_eq("age", QueryValue::Int(25))
    .project(&["name", "email"])
    .limit(100);

// Build logical plan
let logical_plan = query.build();

// Optimize
let optimizer = QueryOptimizer::new();
let physical_plan = optimizer.optimize(logical_plan)?;

// Execute
use tokitai_context::query_optimizer::QueryExecutor;
use std::sync::Arc;

let executor = QueryExecutor::new(Arc::new(optimizer));
let results = executor.execute(physical_plan).await?;

println!("Found {} rows", results.rows_affected);
```

### Complex Query with Join

```rust
use tokitai_context::query_optimizer::{
    Query, QueryValue, QueryPredicate, AggregateFunction,
    JoinType, JoinCondition, SortOrder,
};

// Build query with multiple operations
let query = Query::scan("orders")
    .filter(QueryPredicate::Gt {
        column: "amount".to_string(),
        value: QueryValue::Float(100.0),
    })
    .project(&["customer_id", "amount", "date"])
    .order_by("date", SortOrder::Desc)
    .limit(1000);

let plan = query.build();
```

### Cost Model Configuration

```rust
use tokitai_context::query_optimizer::{
    QueryOptimizer, OptimizerConfig, CostModel, TableStatistics,
};

// Configure optimizer
let config = OptimizerConfig {
    cost_based_optimization: true,
    predicate_pushdown: true,
    projection_pushdown: true,
    join_reordering: true,
    parallel_execution: true,
    plan_caching: true,
    max_plans_explored: 1000,
    cost_threshold: 10000.0,
};

let optimizer = QueryOptimizer::with_config(config);

// Register table statistics
let table_stats = TableStatistics {
    row_count: 1000000,
    page_count: 10000,
    avg_row_size: 256,
    ..Default::default()
};

optimizer.register_table_stats("users", table_stats);
```

## Performance Considerations

### Cost Parameters
- `seq_page_cost`: Cost per sequential page read (default: 1.0)
- `random_page_cost`: Cost per random page read (default: 4.0)
- `cpu_tuple_cost`: Cost per row processing (default: 0.01)
- `cpu_operator_cost`: Cost per operator evaluation (default: 0.0025)

### Plan Caching
- Automatically caches optimized plans
- Cache key based on query structure hash
- Clear cache when statistics change significantly

### Statistics Collection
- Maintain up-to-date table statistics
- Use histograms for range selectivity
- Track column value distributions

## Testing

The module includes comprehensive tests:
- Query builder functionality
- Predicate evaluation
- Cost estimation
- Plan node creation
- Executor basic operations

Run tests:
```bash
cargo test --lib query_optimizer::tests
```

## Future Enhancements

1. **Advanced Statistics**: Multi-column statistics, correlation tracking
2. **Adaptive Optimization**: Runtime plan adjustment based on actual costs
3. **Parallel Execution**: Multi-threaded query execution
4. **Materialized Views**: Automatic view selection and maintenance
5. **Query Hints**: User-provided optimization hints

## References

- PostgreSQL Query Planner: https://www.postgresql.org/docs/current/planner-usage.html
- Cost-Based Query Optimization: https://db.cs.berkeley.edu/cs286/papers/cbo-tutorial.pdf
- Join Ordering Algorithms: https://www.vldb.org/pvldb/vol10/p101-schmidt.pdf
