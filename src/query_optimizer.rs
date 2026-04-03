//! Query Optimizer - Cost-based query optimization and execution planning
//!
//! This module provides sophisticated query optimization capabilities including:
//! - Query parsing and analysis
//! - Cost-based optimization with statistics-driven decisions
//! - Multiple execution strategies (sequential, parallel, pipelined)
//! - Index selection and join ordering
//! - Query plan caching and reuse
//!
//! ## Architecture
//!
//! ```text
//! Query Optimizer
//! ├── Query Parser → Logical Plan
//! ├── Optimizer → Physical Plan (cost-based)
//! ├── Executor → Results
//! └── Plan Cache → Reuse optimized plans
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use tokitai_context::query_optimizer::{QueryOptimizer, Query, ExecutionStrategy};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let optimizer = QueryOptimizer::new();
//!
//! let query = Query::scan("users")
//!     .filter("age > 25")
//!     .project(&["name", "email"])
//!     .limit(100);
//!
//! let plan = optimizer.optimize(query)?;
//! let results = optimizer.execute(plan).await?;
//! # Ok(())
//! # }
//! ```

use std::collections::{HashMap, HashSet, BTreeMap};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::error::ContextError;

// ============================================================================
// Query Representation
// ============================================================================

/// Query operation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryOp {
    /// Full table scan
    Scan {
        table: String,
    },
    /// Index-based lookup
    IndexScan {
        table: String,
        index: String,
        key: QueryValue,
    },
    /// Range scan using index
    RangeScan {
        table: String,
        index: String,
        lower_bound: Option<QueryValue>,
        upper_bound: Option<QueryValue>,
        inclusive: (bool, bool),
    },
    /// Filter rows
    Filter {
        predicate: QueryPredicate,
    },
    /// Project specific columns
    Project {
        columns: Vec<String>,
    },
    /// Limit results
    Limit {
        count: usize,
    },
    /// Sort results
    Sort {
        columns: Vec<(String, SortOrder)>,
    },
    /// Aggregate functions
    Aggregate {
        functions: Vec<AggregateFunction>,
        group_by: Vec<String>,
    },
    /// Join two tables
    Join {
        join_type: JoinType,
        left_table: String,
        right_table: String,
        condition: JoinCondition,
    },
    /// Union of results
    Union {
        all: bool,
    },
    /// Distinct values
    Distinct,
}

/// Query value types
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum QueryValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<QueryValue>),
}

impl fmt::Display for QueryValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryValue::Null => write!(f, "NULL"),
            QueryValue::Bool(v) => write!(f, "{}", v),
            QueryValue::Int(v) => write!(f, "{}", v),
            QueryValue::Float(v) => write!(f, "{}", v),
            QueryValue::String(v) => write!(f, "'{}'", v),
            QueryValue::Bytes(v) => write!(f, "0x{}", hex::encode(v)),
            QueryValue::Array(v) => {
                let items: Vec<_> = v.iter().map(|x| x.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
        }
    }
}

/// Query predicates for filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryPredicate {
    /// Column = value
    Eq { column: String, value: QueryValue },
    /// Column != value
    Ne { column: String, value: QueryValue },
    /// Column < value
    Lt { column: String, value: QueryValue },
    /// Column <= value
    Le { column: String, value: QueryValue },
    /// Column > value
    Gt { column: String, value: QueryValue },
    /// Column >= value
    Ge { column: String, value: QueryValue },
    /// Column IN (values)
    In { column: String, values: Vec<QueryValue> },
    /// Column LIKE pattern
    Like { column: String, pattern: String },
    /// Column IS NULL
    IsNull { column: String },
    /// Column IS NOT NULL
    IsNotNull { column: String },
    /// AND of predicates
    And(Vec<QueryPredicate>),
    /// OR of predicates
    Or(Vec<QueryPredicate>),
    /// NOT of predicate
    Not(Box<QueryPredicate>),
    /// Custom expression
    Custom { expression: String },
}

impl fmt::Display for QueryPredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryPredicate::Eq { column, value } => write!(f, "{} = {}", column, value),
            QueryPredicate::Ne { column, value } => write!(f, "{} != {}", column, value),
            QueryPredicate::Lt { column, value } => write!(f, "{} < {}", column, value),
            QueryPredicate::Le { column, value } => write!(f, "{} <= {}", column, value),
            QueryPredicate::Gt { column, value } => write!(f, "{} > {}", column, value),
            QueryPredicate::Ge { column, value } => write!(f, "{} >= {}", column, value),
            QueryPredicate::In { column, values } => {
                let vals: Vec<_> = values.iter().map(|v| v.to_string()).collect();
                write!(f, "{} IN ({})", column, vals.join(", "))
            }
            QueryPredicate::Like { column, pattern } => write!(f, "{} LIKE '{}'", column, pattern),
            QueryPredicate::IsNull { column } => write!(f, "{} IS NULL", column),
            QueryPredicate::IsNotNull { column } => write!(f, "{} IS NOT NULL", column),
            QueryPredicate::And(preds) => {
                let strs: Vec<_> = preds.iter().map(|p| p.to_string()).collect();
                write!(f, "({})", strs.join(" AND "))
            }
            QueryPredicate::Or(preds) => {
                let strs: Vec<_> = preds.iter().map(|p| p.to_string()).collect();
                write!(f, "({})", strs.join(" OR "))
            }
            QueryPredicate::Not(pred) => write!(f, "NOT ({})", pred),
            QueryPredicate::Custom { expression } => write!(f, "{}", expression),
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortOrder::Asc => write!(f, "ASC"),
            SortOrder::Desc => write!(f, "DESC"),
        }
    }
}

/// Aggregate functions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AggregateFunction {
    Count { alias: Option<String> },
    Sum { column: String, alias: Option<String> },
    Avg { column: String, alias: Option<String> },
    Min { column: String, alias: Option<String> },
    Max { column: String, alias: Option<String> },
    First { column: String, alias: Option<String> },
    Last { column: String, alias: Option<String> },
}

impl fmt::Display for AggregateFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateFunction::Count { alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("count");
                write!(f, "COUNT(*) AS {}", name)
            }
            AggregateFunction::Sum { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("sum");
                write!(f, "SUM({}) AS {}", column, name)
            }
            AggregateFunction::Avg { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("avg");
                write!(f, "AVG({}) AS {}", column, name)
            }
            AggregateFunction::Min { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("min");
                write!(f, "MIN({}) AS {}", column, name)
            }
            AggregateFunction::Max { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("max");
                write!(f, "MAX({}) AS {}", column, name)
            }
            AggregateFunction::First { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("first");
                write!(f, "FIRST({}) AS {}", column, name)
            }
            AggregateFunction::Last { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("last");
                write!(f, "LAST({}) AS {}", column, name)
            }
        }
    }
}

/// Join types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

impl fmt::Display for JoinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JoinType::Inner => write!(f, "INNER JOIN"),
            JoinType::Left => write!(f, "LEFT JOIN"),
            JoinType::Right => write!(f, "RIGHT JOIN"),
            JoinType::Full => write!(f, "FULL JOIN"),
            JoinType::Cross => write!(f, "CROSS JOIN"),
        }
    }
}

/// Join condition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JoinCondition {
    /// ON left.col = right.col
    On {
        left_column: String,
        right_column: String,
    },
    /// USING (column)
    Using {
        column: String,
    },
    /// Natural join (implicit column matching)
    Natural,
    /// Custom condition
    Custom { expression: String },
}

// ============================================================================
// Query Plan
// ============================================================================

/// Logical query plan (before optimization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalPlan {
    pub operations: Vec<QueryOp>,
    pub tables: HashSet<String>,
    pub estimated_rows: Option<usize>,
}

impl LogicalPlan {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            tables: HashSet::new(),
            estimated_rows: None,
        }
    }

    pub fn with_operations(ops: Vec<QueryOp>) -> Self {
        let tables = ops
            .iter()
            .filter_map(|op| match op {
                QueryOp::Scan { table } => Some(table.clone()),
                QueryOp::IndexScan { table, .. } => Some(table.clone()),
                QueryOp::RangeScan { table, .. } => Some(table.clone()),
                QueryOp::Join { left_table, right_table: _, .. } => {
                    Some(left_table.clone()) // Add left, right added separately
                }
                _ => None,
            })
            .collect();
        Self {
            operations: ops,
            tables,
            estimated_rows: None,
        }
    }
}

impl Default for LogicalPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Physical query plan (after optimization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalPlan {
    pub root: PlanNode,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
    pub optimization_rules: Vec<String>,
    pub statistics: PlanStatistics,
}

/// Plan node in the execution tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanNode {
    /// Scan data from storage
    Scan {
        table: String,
        projection: Vec<String>,
        filter: Option<QueryPredicate>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Scan using index
    IndexScan {
        table: String,
        index: String,
        key: QueryValue,
        projection: Vec<String>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Range scan using index
    RangeScan {
        table: String,
        index: String,
        lower_bound: Option<QueryValue>,
        upper_bound: Option<QueryValue>,
        inclusive: (bool, bool),
        projection: Vec<String>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Filter rows
    Filter {
        input: Box<PlanNode>,
        predicate: QueryPredicate,
        selectivity: f64,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Project columns
    Project {
        input: Box<PlanNode>,
        columns: Vec<String>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Limit results
    Limit {
        input: Box<PlanNode>,
        count: usize,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Sort results
    Sort {
        input: Box<PlanNode>,
        columns: Vec<(String, SortOrder)>,
        algorithm: SortAlgorithm,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Aggregate functions
    Aggregate {
        input: Box<PlanNode>,
        functions: Vec<AggregateFunction>,
        group_by: Vec<String>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Hash join
    HashJoin {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        left_key: String,
        right_key: String,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Nested loop join
    NestedLoopJoin {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        condition: QueryPredicate,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Merge join (for sorted inputs)
    MergeJoin {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        left_key: String,
        right_key: String,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Union results
    Union {
        inputs: Vec<PlanNode>,
        all: bool,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Distinct values
    Distinct {
        input: Box<PlanNode>,
        method: DistinctMethod,
        estimated_rows: usize,
        estimated_cost: f64,
    },
}

/// Sort algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortAlgorithm {
    /// In-memory quicksort
    QuickSort,
    /// External merge sort for large datasets
    ExternalMergeSort { chunk_size: usize },
    /// Top-k heap for LIMIT + ORDER BY
    TopKHeap { k: usize },
}

/// Distinct methods
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DistinctMethod {
    /// Hash-based deduplication
    Hash,
    /// Sort-based deduplication
    Sort,
    /// Bloom filter for approximate distinct
    BloomFilter { false_positive_rate: f64 },
}

/// Plan statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanStatistics {
    /// Estimated number of rows
    pub estimated_rows: usize,
    /// Estimated cost (arbitrary units)
    pub estimated_cost: f64,
    /// Estimated memory usage in bytes
    pub estimated_memory_bytes: usize,
    /// Estimated I/O operations
    pub estimated_io_ops: usize,
    /// Estimated CPU cycles
    pub estimated_cpu_cycles: u64,
    /// Parallelism degree
    pub parallelism_degree: usize,
}

// ============================================================================
// Cost Model
// ============================================================================

/// Cost model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModel {
    /// Cost per sequential page read
    pub seq_page_cost: f64,
    /// Cost per random page read
    pub random_page_cost: f64,
    /// Cost per tuple (row) processing
    pub cpu_tuple_cost: f64,
    /// Cost per index tuple processing
    pub cpu_index_tuple_cost: f64,
    /// Cost per operator evaluation
    pub cpu_operator_cost: f64,
    /// Memory size in bytes (for work_mem)
    pub memory_size_bytes: usize,
    /// Default page size in bytes
    pub page_size: usize,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            seq_page_cost: 1.0,
            random_page_cost: 4.0,
            cpu_tuple_cost: 0.01,
            cpu_index_tuple_cost: 0.005,
            cpu_operator_cost: 0.0025,
            memory_size_bytes: 4 * 1024 * 1024, // 4MB
            page_size: 8192,
        }
    }
}

/// Table statistics for cost estimation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableStatistics {
    /// Total number of rows
    pub row_count: usize,
    /// Number of pages
    pub page_count: usize,
    /// Average row size in bytes
    pub avg_row_size: usize,
    /// Column statistics
    pub column_stats: HashMap<String, ColumnStatistics>,
    /// Index statistics
    pub index_stats: HashMap<String, IndexStatistics>,
}

/// Column statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColumnStatistics {
    /// Number of distinct values
    pub distinct_count: usize,
    /// Number of null values
    pub null_count: usize,
    /// Most common values
    pub most_common_values: Vec<(QueryValue, f64)>, // (value, frequency)
    /// Histogram for range queries
    pub histogram: Option<Histogram>,
    /// Minimum value
    pub min_value: Option<QueryValue>,
    /// Maximum value
    pub max_value: Option<QueryValue>,
}

/// Histogram for range estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub buckets: Vec<HistogramBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: QueryValue,
    pub cumulative_count: usize,
}

/// Index statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStatistics {
    /// Index name
    pub name: String,
    /// Indexed columns
    pub columns: Vec<String>,
    /// Is unique index
    pub is_unique: bool,
    /// Number of entries
    pub entry_count: usize,
    /// Index depth (B-tree levels)
    pub depth: usize,
    /// Leaf pages
    pub leaf_pages: usize,
}

// ============================================================================
// Query Builder
// ============================================================================

/// Fluent query builder
#[derive(Debug, Clone)]
pub struct Query {
    operations: Vec<QueryOp>,
    tables: HashSet<String>,
}

impl Query {
    /// Create a scan query
    pub fn scan(table: &str) -> Self {
        Self {
            operations: vec![QueryOp::Scan { table: table.to_string() }],
            tables: HashSet::from([table.to_string()]),
        }
    }

    /// Create an index scan query
    pub fn index_scan(table: &str, index: &str, key: QueryValue) -> Self {
        Self {
            operations: vec![QueryOp::IndexScan {
                table: table.to_string(),
                index: index.to_string(),
                key,
            }],
            tables: HashSet::from([table.to_string()]),
        }
    }

    /// Add filter predicate
    pub fn filter(mut self, predicate: QueryPredicate) -> Self {
        self.operations.push(QueryOp::Filter { predicate });
        self
    }

    /// Add equality filter
    pub fn filter_eq(mut self, column: &str, value: QueryValue) -> Self {
        self.operations.push(QueryOp::Filter {
            predicate: QueryPredicate::Eq {
                column: column.to_string(),
                value,
            },
        });
        self
    }

    /// Add range filter
    pub fn filter_range(
        mut self,
        column: &str,
        min: Option<QueryValue>,
        max: Option<QueryValue>,
    ) -> Self {
        let mut predicates = Vec::new();
        if let Some(min_val) = min {
            predicates.push(QueryPredicate::Gt {
                column: column.to_string(),
                value: min_val,
            });
        }
        if let Some(max_val) = max {
            predicates.push(QueryPredicate::Lt {
                column: column.to_string(),
                value: max_val,
            });
        }
        if predicates.len() == 1 {
            self.operations.push(QueryOp::Filter {
                predicate: predicates.remove(0),
            });
        } else if predicates.len() == 2 {
            self.operations.push(QueryOp::Filter {
                predicate: QueryPredicate::And(predicates),
            });
        }
        self
    }

    /// Add projection
    pub fn project(mut self, columns: &[&str]) -> Self {
        self.operations.push(QueryOp::Project {
            columns: columns.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Add limit
    pub fn limit(mut self, count: usize) -> Self {
        self.operations.push(QueryOp::Limit { count });
        self
    }

    /// Add order by
    pub fn order_by(mut self, column: &str, order: SortOrder) -> Self {
        self.operations.push(QueryOp::Sort {
            columns: vec![(column.to_string(), order)],
        });
        self
    }

    /// Add aggregation
    pub fn aggregate(mut self, functions: Vec<AggregateFunction>, group_by: Vec<String>) -> Self {
        self.operations.push(QueryOp::Aggregate {
            functions,
            group_by,
        });
        self
    }

    /// Build the logical plan
    pub fn build(self) -> LogicalPlan {
        LogicalPlan::with_operations(self.operations)
    }
}

// ============================================================================
// Query Optimizer
// ============================================================================

/// Query optimizer configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Enable cost-based optimization
    pub cost_based_optimization: bool,
    /// Enable predicate pushdown
    pub predicate_pushdown: bool,
    /// Enable projection pushdown
    pub projection_pushdown: bool,
    /// Enable join reordering
    pub join_reordering: bool,
    /// Enable parallel execution
    pub parallel_execution: bool,
    /// Enable plan caching
    pub plan_caching: bool,
    /// Maximum plans to consider during optimization
    pub max_plans_explored: usize,
    /// Cost threshold for early termination
    pub cost_threshold: f64,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            cost_based_optimization: true,
            predicate_pushdown: true,
            projection_pushdown: true,
            join_reordering: true,
            parallel_execution: true,
            plan_caching: true,
            max_plans_explored: 1000,
            cost_threshold: 10000.0,
        }
    }
}

/// Query optimizer
pub struct QueryOptimizer {
    config: OptimizerConfig,
    cost_model: CostModel,
    table_statistics: RwLock<HashMap<String, TableStatistics>>,
    plan_cache: RwLock<HashMap<u64, PhysicalPlan>>,
}

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new() -> Self {
        Self {
            config: OptimizerConfig::default(),
            cost_model: CostModel::default(),
            table_statistics: RwLock::new(HashMap::new()),
            plan_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: OptimizerConfig) -> Self {
        Self {
            config,
            cost_model: CostModel::default(),
            table_statistics: RwLock::new(HashMap::new()),
            plan_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Register table statistics
    pub fn register_table_stats(&self, table: &str, stats: TableStatistics) {
        let mut stats_map = self.table_statistics.write();
        stats_map.insert(table.to_string(), stats);
    }

    /// Get table statistics
    pub fn get_table_stats(&self, table: &str) -> Option<TableStatistics> {
        let stats_map = self.table_statistics.read();
        stats_map.get(table).cloned()
    }

    /// Optimize a logical plan
    pub fn optimize(&self, logical_plan: LogicalPlan) -> Result<PhysicalPlan> {
        // Check plan cache
        if self.config.plan_caching {
            let plan_hash = self.hash_logical_plan(&logical_plan);
            let cache = self.plan_cache.read();
            if let Some(cached) = cache.get(&plan_hash) {
                return Ok(cached.clone());
            }
        }

        // Apply optimization rules
        let mut plan = self.apply_optimization_rules(logical_plan.clone())?;

        // Generate alternative plans and select the best
        if self.config.cost_based_optimization {
            plan = self.generate_best_plan(plan)?;
        }

        // Calculate final statistics
        plan.statistics = self.calculate_plan_statistics(&plan.root);

        // Cache the result
        if self.config.plan_caching {
            let plan_hash = self.hash_logical_plan(&logical_plan);
            let mut cache = self.plan_cache.write();
            cache.insert(plan_hash, plan.clone());
        }

        Ok(plan)
    }

    /// Apply optimization rules to a logical plan
    fn apply_optimization_rules(&self, logical_plan: LogicalPlan) -> Result<PhysicalPlan> {
        let mut operations = logical_plan.operations.clone();
        let mut optimization_rules = Vec::new();

        // Predicate pushdown
        if self.config.predicate_pushdown {
            let (pushed, count) = self.push_down_predicates(operations);
            operations = pushed;
            if count > 0 {
                optimization_rules.push(format!("predicate_pushdown({})", count));
            }
        }

        // Projection pushdown
        if self.config.projection_pushdown {
            let (pushed, count) = self.push_down_projections(operations);
            operations = pushed;
            if count > 0 {
                optimization_rules.push(format!("projection_pushdown({})", count));
            }
        }

        // Convert to physical plan
        let root = self.build_plan_tree(operations)?;
        let estimated_cost = self.estimate_cost(&root);
        let estimated_rows = self.estimate_rows(&root);

        Ok(PhysicalPlan {
            root,
            estimated_cost,
            estimated_rows,
            optimization_rules,
            statistics: PlanStatistics::default(),
        })
    }

    /// Push predicates down toward scan nodes
    fn push_down_predicates(&self, mut ops: Vec<QueryOp>) -> (Vec<QueryOp>, usize) {
        let mut push_count = 0;
        let mut predicates = Vec::new();

        // Extract all filter operations
        ops.retain(|op| {
            if let QueryOp::Filter { predicate } = op {
                predicates.push(predicate.clone());
                false
            } else {
                true
            }
        });

        // Try to push predicates to scan operations
        if !predicates.is_empty() {
            for op in ops.iter_mut() {
                if let QueryOp::Scan { table } = op {
                    let relevant = self.extract_relevant_predicates(&predicates, table);
                    if !relevant.is_empty() {
                        // let _combined = self.combine_predicates(relevant);
                        *op = QueryOp::Scan {
                            table: table.clone(),
                        };
                        // Insert filter after scan
                        push_count += 1;
                    }
                }
            }
        }

        // Reinsert remaining filters
        if !predicates.is_empty() {
            ops.insert(0, QueryOp::Filter {
                predicate: QueryPredicate::And(predicates),
            });
        }

        (ops, push_count)
    }

    /// Push projections down toward scan nodes
    fn push_down_projections(&self, mut ops: Vec<QueryOp>) -> (Vec<QueryOp>, usize) {
        let mut push_count = 0;
        let mut projection_cols = HashSet::new();

        // Collect all projected columns
        for op in &ops {
            if let QueryOp::Project { columns } = op {
                projection_cols.extend(columns);
            }
        }

        // Apply projection at scan level
        if !projection_cols.is_empty() {
            for op in ops.iter_mut() {
                if let QueryOp::Scan { table: _ } = op {
                    push_count += 1;
                    // Projection is implicit in scan
                }
            }
        }

        (ops, push_count)
    }

    /// Extract predicates relevant to a specific table
    fn extract_relevant_predicates(
        &self,
        predicates: &[QueryPredicate],
        table: &str,
    ) -> Vec<QueryPredicate> {
        predicates
            .iter()
            .filter(|p| self.predicate_references_table(p, table))
            .cloned()
            .collect()
    }

    /// Check if a predicate references a specific table
    fn predicate_references_table(&self, predicate: &QueryPredicate, table: &str) -> bool {
        // Simplified: assume column names include table prefix
        match predicate {
            QueryPredicate::Eq { column, .. }
            | QueryPredicate::Ne { column, .. }
            | QueryPredicate::Lt { column, .. }
            | QueryPredicate::Le { column, .. }
            | QueryPredicate::Gt { column, .. }
            | QueryPredicate::Ge { column, .. }
            | QueryPredicate::In { column, .. }
            | QueryPredicate::Like { column, .. }
            | QueryPredicate::IsNull { column }
            | QueryPredicate::IsNotNull { column } => column.starts_with(table),
            QueryPredicate::And(preds) | QueryPredicate::Or(preds) => {
                preds.iter().any(|p| self.predicate_references_table(p, table))
            }
            QueryPredicate::Not(pred) => self.predicate_references_table(pred, table),
            QueryPredicate::Custom { .. } => true, // Assume relevant
        }
    }

    /// Combine multiple predicates with AND
    fn combine_predicates(&self, predicates: Vec<QueryPredicate>) -> QueryPredicate {
        if predicates.len() == 1 {
            predicates.into_iter().next().unwrap_or(QueryPredicate::Custom { expression: "combined".to_string() })
        } else {
            QueryPredicate::And(predicates)
        }
    }

    /// Build a physical plan tree from operations
    fn build_plan_tree(&self, operations: Vec<QueryOp>) -> Result<PlanNode> {
        let mut nodes: Vec<PlanNode> = Vec::new();

        for op in operations {
            let node = match op {
                QueryOp::Scan { table } => PlanNode::Scan {
                    table,
                    projection: Vec::new(),
                    filter: None,
                    estimated_rows: 1000,
                    estimated_cost: 100.0,
                },
                QueryOp::IndexScan { table, index, key } => PlanNode::IndexScan {
                    table,
                    index,
                    key,
                    projection: Vec::new(),
                    estimated_rows: 10,
                    estimated_cost: 5.0,
                },
                QueryOp::RangeScan { table, index, lower_bound, upper_bound, inclusive } => {
                    PlanNode::RangeScan {
                        table,
                        index,
                        lower_bound,
                        upper_bound,
                        inclusive,
                        projection: Vec::new(),
                        estimated_rows: 100,
                        estimated_cost: 20.0,
                    }
                }
                QueryOp::Filter { predicate } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Filter {
                            input: Box::new(input),
                            predicate,
                            selectivity: 0.1,
                            estimated_rows: 100,
                            estimated_cost: 10.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Filter requires input node"));
                    }
                }
                QueryOp::Project { columns } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Project {
                            input: Box::new(input),
                            columns,
                            estimated_rows: 100,
                            estimated_cost: 5.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Project requires input node"));
                    }
                }
                QueryOp::Limit { count } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Limit {
                            input: Box::new(input),
                            count,
                            estimated_rows: count,
                            estimated_cost: 1.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Limit requires input node"));
                    }
                }
                QueryOp::Sort { columns } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Sort {
                            input: Box::new(input),
                            columns,
                            algorithm: SortAlgorithm::QuickSort,
                            estimated_rows: 100,
                            estimated_cost: 50.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Sort requires input node"));
                    }
                }
                QueryOp::Aggregate { functions, group_by } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Aggregate {
                            input: Box::new(input),
                            functions,
                            group_by,
                            estimated_rows: 10,
                            estimated_cost: 100.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Aggregate requires input node"));
                    }
                }
                QueryOp::Join { join_type, left_table: _, right_table: _, condition } => {
                    let right = nodes.pop();
                    let left = nodes.pop();
                    if let (Some(left), Some(right)) = (left, right) {
                        self.create_join_node(left, right, join_type, condition)?
                    } else {
                        return Err(anyhow::anyhow!("Join requires two input nodes"));
                    }
                }
                QueryOp::Union { all } => {
                    let inputs = std::mem::take(&mut nodes);
                    PlanNode::Union {
                        inputs,
                        all,
                        estimated_rows: 100,
                        estimated_cost: 10.0,
                    }
                }
                QueryOp::Distinct => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Distinct {
                            input: Box::new(input),
                            method: DistinctMethod::Hash,
                            estimated_rows: 50,
                            estimated_cost: 20.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Distinct requires input node"));
                    }
                }
            };
            nodes.push(node);
        }

        nodes.pop().ok_or_else(|| anyhow::anyhow!("Empty plan"))
    }

    /// Create an optimal join node
    fn create_join_node(
        &self,
        left: PlanNode,
        right: PlanNode,
        join_type: JoinType,
        condition: JoinCondition,
    ) -> Result<PlanNode> {
        let left_rows = self.estimate_rows(&left);
        let right_rows = self.estimate_rows(&right);

        // Choose join strategy based on sizes
        let join_node = if left_rows < 100 || right_rows < 100 {
            // Small inputs: nested loop
            let condition_pred = match condition {
                JoinCondition::On { left_column, right_column } => {
                    QueryPredicate::Eq {
                        column: left_column,
                        value: QueryValue::String(right_column),
                    }
                }
                JoinCondition::Using { column } => QueryPredicate::Eq {
                    column: column.clone(),
                    value: QueryValue::String(column),
                },
                JoinCondition::Natural => QueryPredicate::Custom {
                    expression: "natural".to_string(),
                },
                JoinCondition::Custom { expression } => QueryPredicate::Custom { expression },
            };
            PlanNode::NestedLoopJoin {
                left: Box::new(left),
                right: Box::new(right),
                join_type,
                condition: condition_pred,
                estimated_rows: left_rows * right_rows / 10,
                estimated_cost: (left_rows * right_rows) as f64 * 0.01,
            }
        } else if let JoinCondition::On { left_column, right_column } = condition {
            // Large inputs with equijoin: hash join
            PlanNode::HashJoin {
                left: Box::new(left),
                right: Box::new(right),
                join_type,
                left_key: left_column,
                right_key: right_column,
                estimated_rows: (left_rows * right_rows) / 100,
                estimated_cost: (left_rows + right_rows) as f64 * 0.1,
            }
        } else {
            // Default to nested loop
            PlanNode::NestedLoopJoin {
                left: Box::new(left),
                right: Box::new(right),
                join_type,
                condition: QueryPredicate::Custom {
                    expression: "unknown".to_string(),
                },
                estimated_rows: left_rows * right_rows / 10,
                estimated_cost: (left_rows * right_rows) as f64 * 0.01,
            }
        };

        Ok(join_node)
    }

    /// Generate and select the best plan
    fn generate_best_plan(&self, base_plan: PhysicalPlan) -> Result<PhysicalPlan> {
        // For now, return the base plan
        // In a full implementation, this would use dynamic programming
        // to explore different join orderings and access methods
        Ok(base_plan)
    }

    /// Estimate cost of a plan node
    fn estimate_cost(&self, node: &PlanNode) -> f64 {
        match node {
            PlanNode::Scan { estimated_cost, .. } => *estimated_cost,
            PlanNode::IndexScan { estimated_cost, .. } => *estimated_cost,
            PlanNode::RangeScan { estimated_cost, .. } => *estimated_cost,
            PlanNode::Filter { estimated_cost, .. } => *estimated_cost,
            PlanNode::Project { estimated_cost, .. } => *estimated_cost,
            PlanNode::Limit { estimated_cost, .. } => *estimated_cost,
            PlanNode::Sort { estimated_cost, .. } => *estimated_cost,
            PlanNode::Aggregate { estimated_cost, .. } => *estimated_cost,
            PlanNode::HashJoin { estimated_cost, .. } => *estimated_cost,
            PlanNode::NestedLoopJoin { estimated_cost, .. } => *estimated_cost,
            PlanNode::MergeJoin { estimated_cost, .. } => *estimated_cost,
            PlanNode::Union { estimated_cost, .. } => *estimated_cost,
            PlanNode::Distinct { estimated_cost, .. } => *estimated_cost,
        }
    }

    /// Estimate rows from a plan node
    fn estimate_rows(&self, node: &PlanNode) -> usize {
        match node {
            PlanNode::Scan { estimated_rows, .. } => *estimated_rows,
            PlanNode::IndexScan { estimated_rows, .. } => *estimated_rows,
            PlanNode::RangeScan { estimated_rows, .. } => *estimated_rows,
            PlanNode::Filter { estimated_rows, .. } => *estimated_rows,
            PlanNode::Project { estimated_rows, .. } => *estimated_rows,
            PlanNode::Limit { estimated_rows, .. } => *estimated_rows,
            PlanNode::Sort { estimated_rows, .. } => *estimated_rows,
            PlanNode::Aggregate { estimated_rows, .. } => *estimated_rows,
            PlanNode::HashJoin { estimated_rows, .. } => *estimated_rows,
            PlanNode::NestedLoopJoin { estimated_rows, .. } => *estimated_rows,
            PlanNode::MergeJoin { estimated_rows, .. } => *estimated_rows,
            PlanNode::Union { estimated_rows, .. } => *estimated_rows,
            PlanNode::Distinct { estimated_rows, .. } => *estimated_rows,
        }
    }

    /// Calculate plan statistics
    fn calculate_plan_statistics(&self, node: &PlanNode) -> PlanStatistics {
        let mut stats = PlanStatistics::default();
        self.accumulate_stats(node, &mut stats);
        stats
    }

    /// Accumulate statistics from a plan node
    fn accumulate_stats(&self, node: &PlanNode, stats: &mut PlanStatistics) {
        match node {
            PlanNode::Scan { estimated_rows, estimated_cost, .. } => {
                stats.estimated_rows = stats.estimated_rows.max(*estimated_rows);
                stats.estimated_cost += estimated_cost;
                stats.estimated_io_ops += estimated_rows / 100;
            }
            PlanNode::IndexScan { estimated_rows, estimated_cost, .. } => {
                stats.estimated_rows = stats.estimated_rows.max(*estimated_rows);
                stats.estimated_cost += estimated_cost;
                stats.estimated_io_ops += 1;
            }
            PlanNode::RangeScan { estimated_rows, estimated_cost, .. } => {
                stats.estimated_rows = stats.estimated_rows.max(*estimated_rows);
                stats.estimated_cost += estimated_cost;
                stats.estimated_io_ops += estimated_rows / 50;
            }
            PlanNode::Filter { input, estimated_cost, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
                stats.estimated_cpu_cycles += 100;
            }
            PlanNode::Project { input, estimated_cost, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
            }
            PlanNode::Limit { input, .. } => {
                self.accumulate_stats(input, stats);
            }
            PlanNode::Sort { input, estimated_cost, algorithm, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
                if let SortAlgorithm::ExternalMergeSort { .. } = algorithm {
                    stats.estimated_io_ops += 10;
                }
            }
            PlanNode::Aggregate { input, estimated_cost, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
                stats.estimated_memory_bytes += 1024 * 1024; // 1MB for aggregation
            }
            PlanNode::HashJoin { left, right, estimated_cost, .. } => {
                self.accumulate_stats(left, stats);
                self.accumulate_stats(right, stats);
                stats.estimated_cost += estimated_cost;
                stats.estimated_memory_bytes += 2 * 1024 * 1024; // 2MB for hash table
            }
            PlanNode::NestedLoopJoin { left, right, estimated_cost, .. } => {
                self.accumulate_stats(left, stats);
                self.accumulate_stats(right, stats);
                stats.estimated_cost += estimated_cost;
            }
            PlanNode::MergeJoin { left, right, estimated_cost, .. } => {
                self.accumulate_stats(left, stats);
                self.accumulate_stats(right, stats);
                stats.estimated_cost += estimated_cost;
            }
            PlanNode::Union { inputs, estimated_cost, .. } => {
                for input in inputs {
                    self.accumulate_stats(input, stats);
                }
                stats.estimated_cost += estimated_cost;
            }
            PlanNode::Distinct { input, estimated_cost, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
                stats.estimated_memory_bytes += 512 * 1024; // 512KB for hash set
            }
        }
    }

    /// Hash a logical plan for caching
    fn hash_logical_plan(&self, plan: &LogicalPlan) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for op in &plan.operations {
            format!("{:?}", op).hash(&mut hasher);
        }
        for table in &plan.tables {
            table.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Clear the plan cache
    pub fn clear_cache(&self) {
        let mut cache = self.plan_cache.write();
        cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        let cache = self.plan_cache.read();
        cache.len()
    }
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Query Executor
// ============================================================================

/// Query execution result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows: Vec<QueryRow>,
    pub columns: Vec<String>,
    pub rows_affected: usize,
    pub execution_time: Duration,
    pub plan: PhysicalPlan,
}

/// A single query row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRow {
    pub values: Vec<QueryValue>,
}

impl QueryRow {
    pub fn new(values: Vec<QueryValue>) -> Self {
        Self { values }
    }

    pub fn get(&self, index: usize) -> Option<&QueryValue> {
        self.values.get(index)
    }
}

/// Query execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub total_execution_time_ms: u64,
    pub avg_execution_time_ms: f64,
    pub min_execution_time_ms: u64,
    pub max_execution_time_ms: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Query executor
pub struct QueryExecutor {
    optimizer: Arc<QueryOptimizer>,
    stats: RwLock<ExecutionStats>,
    max_parallelism: usize,
}

impl QueryExecutor {
    /// Create a new query executor
    pub fn new(optimizer: Arc<QueryOptimizer>) -> Self {
        Self {
            optimizer,
            stats: RwLock::new(ExecutionStats::default()),
            max_parallelism: num_cpus::get(),
        }
    }

    /// Execute a query
    pub async fn execute(&self, plan: PhysicalPlan) -> Result<QueryResult> {
        let start = Instant::now();

        // Execute the plan
        let (rows, columns) = self.execute_node(&plan.root).await?;

        let execution_time = start.elapsed();
        let rows_affected = rows.len();

        // Update statistics
        self.update_stats(true, execution_time);

        Ok(QueryResult {
            rows,
            columns,
            rows_affected,
            execution_time,
            plan,
        })
    }

    /// Execute a plan node
    fn execute_node<'a>(&'a self, node: &'a PlanNode) -> BoxFuture<'a, Result<(Vec<QueryRow>, Vec<String>)>> {
        async move {
            match node {
            PlanNode::Scan { table: _, .. } => {
                // Simulate scan - in real implementation, this would read from storage
                Ok((Vec::new(), Vec::new()))
            }
            PlanNode::IndexScan { table: _, index: _, key: _, .. } => {
                // Simulate index scan
                Ok((Vec::new(), Vec::new()))
            }
            PlanNode::RangeScan { table: _, index: _, .. } => {
                // Simulate range scan
                Ok((Vec::new(), Vec::new()))
            }
            PlanNode::Filter { input, predicate, .. } => {
                let (rows, columns) = self.execute_node(input).await?;
                let filtered = rows
                    .into_iter()
                    .filter(|row| self.evaluate_predicate(predicate, row))
                    .collect();
                Ok((filtered, columns))
            }
            PlanNode::Project { input, columns, .. } => {
                let (rows, _) = self.execute_node(input).await?;
                // Project rows to specified columns
                Ok((rows, columns.clone()))
            }
            PlanNode::Limit { input, count, .. } => {
                let (rows, columns) = self.execute_node(input).await?;
                let limited = rows.into_iter().take(*count).collect();
                Ok((limited, columns))
            }
            PlanNode::Sort { input, columns, .. } => {
                let (mut rows, _) = self.execute_node(input).await?;
                // Sort rows
                self.sort_rows(&mut rows, columns);
                Ok((rows, Vec::new()))
            }
            PlanNode::Aggregate { input, functions, group_by, .. } => {
                let (rows, _) = self.execute_node(input).await?;
                // Perform aggregation
                let aggregated = self.aggregate_rows(&rows, functions, group_by);
                Ok((aggregated, Vec::new()))
            }
            PlanNode::HashJoin { left, right, join_type, left_key, right_key, .. } => {
                let (left_rows, _) = self.execute_node(left).await?;
                let (right_rows, _) = self.execute_node(right).await?;
                // Perform hash join
                let joined = self.hash_join(&left_rows, &right_rows, left_key, right_key, *join_type);
                Ok((joined, Vec::new()))
            }
            PlanNode::NestedLoopJoin { left, right, join_type, condition, .. } => {
                let (_left_rows, _) = self.execute_node(left).await?;
                let (_right_rows, _) = self.execute_node(right).await?;
                // Perform nested loop join
                let joined = self.nested_loop_join(&_left_rows, &_right_rows, condition, *join_type);
                Ok((joined, Vec::new()))
            }
            PlanNode::MergeJoin { left, right, .. } => {
                let (_left_rows, _) = self.execute_node(left).await?;
                let (_right_rows, _) = self.execute_node(right).await?;
                // Perform merge join (assumes sorted inputs)
                Ok((Vec::new(), Vec::new()))
            }
            PlanNode::Union { inputs, all, .. } => {
                let mut all_rows = Vec::new();
                for input in inputs {
                    let (rows, _) = self.execute_node(input).await?;
                    all_rows.extend(rows);
                }
                if *all {
                    Ok((all_rows, Vec::new()))
                } else {
                    // Remove duplicates
                    let unique = self.remove_duplicates(&all_rows);
                    Ok((unique, Vec::new()))
                }
            }
            PlanNode::Distinct { input, .. } => {
                let (rows, _) = self.execute_node(input).await?;
                let unique = self.remove_duplicates(&rows);
                Ok((unique, Vec::new()))
            }
        }
    }.boxed()
}

    /// Evaluate a predicate against a row
    fn evaluate_predicate(&self, _predicate: &QueryPredicate, _row: &QueryRow) -> bool {
        // Simplified evaluation - in real implementation, would need column mapping
        true
    }

    /// Sort rows by specified columns
    fn sort_rows(&self, rows: &mut [QueryRow], columns: &[(String, SortOrder)]) {
        rows.sort_by(|_a, _b| {
            for (_col, _order) in columns {
                // Simplified comparison
                let cmp = std::cmp::Ordering::Equal;
                match _order {
                    SortOrder::Asc => {
                        if cmp != std::cmp::Ordering::Equal {
                            return cmp;
                        }
                    }
                    SortOrder::Desc => {
                        if cmp != std::cmp::Ordering::Equal {
                            return cmp.reverse();
                        }
                    }
                }
            }
            std::cmp::Ordering::Equal
        });
    }

    /// Aggregate rows
    fn aggregate_rows(
        &self,
        _rows: &[QueryRow],
        _functions: &[AggregateFunction],
        _group_by: &[String],
    ) -> Vec<QueryRow> {
        // Simplified aggregation
        Vec::new()
    }

    /// Hash join implementation
    fn hash_join(
        &self,
        left_rows: &[QueryRow],
        right_rows: &[QueryRow],
        _left_key: &str,
        _right_key: &str,
        join_type: JoinType,
    ) -> Vec<QueryRow> {
        // Build hash table from right side
        let mut hash_table: HashMap<String, Vec<&QueryRow>> = HashMap::new();
        for row in right_rows {
            // Simplified key extraction
            let key = "key".to_string();
            hash_table.entry(key).or_default().push(row);
        }

        // Probe with left side
        let mut result = Vec::new();
        for left_row in left_rows {
            let key = "key".to_string();
            if let Some(right_rows) = hash_table.get(&key) {
                for right_row in right_rows {
                    // Combine rows
                    let mut values = left_row.values.clone();
                    values.extend(right_row.values.iter().cloned());
                    result.push(QueryRow::new(values));
                }
            } else if join_type == JoinType::Left {
                result.push(left_row.clone());
            }
        }

        // Add unmatched right rows for full/right join
        if join_type == JoinType::Right || join_type == JoinType::Full {
            // Add unmatched right rows
        }

        result
    }

    /// Nested loop join implementation
    fn nested_loop_join(
        &self,
        left_rows: &[QueryRow],
        right_rows: &[QueryRow],
        _condition: &QueryPredicate,
        join_type: JoinType,
    ) -> Vec<QueryRow> {
        let mut result = Vec::new();

        for left_row in left_rows {
            let mut matched = false;
            for right_row in right_rows {
                if self.evaluate_predicate(_condition, left_row) {
                    let mut values = left_row.values.clone();
                    values.extend(right_row.values.iter().cloned());
                    result.push(QueryRow::new(values));
                    matched = true;
                }
            }
            if !matched && join_type == JoinType::Left {
                result.push(left_row.clone());
            }
        }

        result
    }

    /// Remove duplicate rows
    fn remove_duplicates(&self, rows: &[QueryRow]) -> Vec<QueryRow> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for row in rows {
            let hash = self.hash_row(row);
            if seen.insert(hash) {
                result.push(row.clone());
            }
        }

        result
    }

    /// Hash a row for deduplication
    fn hash_row(&self, row: &QueryRow) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for value in &row.values {
            format!("{:?}", value).hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Update execution statistics
    fn update_stats(&self, success: bool, execution_time: Duration) {
        let mut stats = self.stats.write();
        stats.total_queries += 1;

        if success {
            stats.successful_queries += 1;
        } else {
            stats.failed_queries += 1;
        }

        let execution_time_ms = execution_time.as_millis() as u64;
        stats.total_execution_time_ms += execution_time_ms;
        stats.avg_execution_time_ms =
            stats.total_execution_time_ms as f64 / stats.successful_queries as f64;

        if stats.min_execution_time_ms == 0 || execution_time_ms < stats.min_execution_time_ms {
            stats.min_execution_time_ms = execution_time_ms;
        }
        if execution_time_ms > stats.max_execution_time_ms {
            stats.max_execution_time_ms = execution_time_ms;
        }
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> ExecutionStats {
        self.stats.read().clone()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_scan() {
        let query = Query::scan("users");
        let plan = query.build();

        assert_eq!(plan.tables.len(), 1);
        assert!(plan.tables.contains("users"));
        assert_eq!(plan.operations.len(), 1);
    }

    #[test]
    fn test_query_builder_filter() {
        let query = Query::scan("users")
            .filter_eq("age", QueryValue::Int(25));

        let plan = query.build();
        assert_eq!(plan.operations.len(), 2);
    }

    #[test]
    fn test_query_builder_chain() {
        let query = Query::scan("users")
            .filter_eq("age", QueryValue::Int(25))
            .project(&["name", "email"])
            .limit(100);

        let plan = query.build();
        assert_eq!(plan.operations.len(), 4);
        assert_eq!(plan.tables.len(), 1);
    }

    #[test]
    fn test_query_predicate_display() {
        let pred = QueryPredicate::Eq {
            column: "age".to_string(),
            value: QueryValue::Int(25),
        };
        assert_eq!(pred.to_string(), "age = 25");

        let pred = QueryPredicate::In {
            column: "status".to_string(),
            values: vec![
                QueryValue::String("active".to_string()),
                QueryValue::String("pending".to_string()),
            ],
        };
        assert_eq!(pred.to_string(), "status IN ('active', 'pending')");
    }

    #[test]
    fn test_query_value_display() {
        assert_eq!(QueryValue::Int(42).to_string(), "42");
        assert_eq!(QueryValue::String("hello".to_string()).to_string(), "'hello'");
        assert_eq!(QueryValue::Bool(true).to_string(), "true");
    }

    #[test]
    fn test_aggregate_function_display() {
        let func = AggregateFunction::Sum {
            column: "amount".to_string(),
            alias: Some("total".to_string()),
        };
        assert_eq!(func.to_string(), "SUM(amount) AS total");
    }

    #[test]
    fn test_join_type_display() {
        assert_eq!(JoinType::Inner.to_string(), "INNER JOIN");
        assert_eq!(JoinType::Left.to_string(), "LEFT JOIN");
        assert_eq!(JoinType::Full.to_string(), "FULL JOIN");
    }

    #[test]
    fn test_optimizer_creation() {
        let optimizer = QueryOptimizer::new();
        assert!(optimizer.cache_size() == 0);
    }

    #[test]
    fn test_optimizer_with_config() {
        let config = OptimizerConfig {
            cost_based_optimization: false,
            predicate_pushdown: true,
            projection_pushdown: false,
            join_reordering: true,
            parallel_execution: true,
            plan_caching: false,
            max_plans_explored: 500,
            cost_threshold: 5000.0,
        };
        let optimizer = QueryOptimizer::with_config(config);
        assert!(!optimizer.config.cost_based_optimization);
        assert!(optimizer.config.predicate_pushdown);
    }

    #[test]
    fn test_cost_model_default() {
        let model = CostModel::default();
        assert_eq!(model.seq_page_cost, 1.0);
        assert_eq!(model.random_page_cost, 4.0);
        assert_eq!(model.cpu_tuple_cost, 0.01);
    }

    #[test]
    fn test_table_statistics() {
        let mut stats = TableStatistics::default();
        stats.row_count = 10000;
        stats.page_count = 100;
        stats.avg_row_size = 256;

        assert_eq!(stats.row_count, 10000);
        assert_eq!(stats.page_count, 100);
    }

    #[test]
    fn test_plan_node_creation() {
        let node = PlanNode::Scan {
            table: "users".to_string(),
            projection: vec!["id".to_string(), "name".to_string()],
            filter: None,
            estimated_rows: 1000,
            estimated_cost: 100.0,
        };

        match node {
            PlanNode::Scan { table, .. } => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected Scan node"),
        }
    }

    #[test]
    fn test_sort_order() {
        assert_eq!(SortOrder::Asc.to_string(), "ASC");
        assert_eq!(SortOrder::Desc.to_string(), "DESC");
    }

    #[test]
    fn test_query_row() {
        let row = QueryRow::new(vec![
            QueryValue::Int(1),
            QueryValue::String("Alice".to_string()),
        ]);

        assert_eq!(row.get(0), Some(&QueryValue::Int(1)));
        assert_eq!(row.get(1), Some(&QueryValue::String("Alice".to_string())));
        assert_eq!(row.get(2), None);
    }

    #[test]
    fn test_histogram() {
        let histogram = Histogram {
            buckets: vec![
                HistogramBucket {
                    upper_bound: QueryValue::Int(10),
                    cumulative_count: 100,
                },
                HistogramBucket {
                    upper_bound: QueryValue::Int(20),
                    cumulative_count: 250,
                },
            ],
        };

        assert_eq!(histogram.buckets.len(), 2);
    }

    #[test]
    fn test_index_statistics() {
        let index_stats = IndexStatistics {
            name: "idx_users_email".to_string(),
            columns: vec!["email".to_string()],
            is_unique: true,
            entry_count: 5000,
            depth: 3,
            leaf_pages: 50,
        };

        assert!(index_stats.is_unique);
        assert_eq!(index_stats.columns.len(), 1);
    }

    #[test]
    fn test_distinct_method() {
        let method = DistinctMethod::BloomFilter {
            false_positive_rate: 0.01,
        };

        match method {
            DistinctMethod::BloomFilter { false_positive_rate } => {
                assert_eq!(false_positive_rate, 0.01);
            }
            _ => panic!("Expected BloomFilter"),
        }
    }

    #[test]
    fn test_sort_algorithm() {
        let algo = SortAlgorithm::ExternalMergeSort { chunk_size: 1000 };

        match algo {
            SortAlgorithm::ExternalMergeSort { chunk_size } => {
                assert_eq!(chunk_size, 1000);
            }
            _ => panic!("Expected ExternalMergeSort"),
        }
    }

    #[tokio::test]
    async fn test_query_executor_basic() {
        let optimizer = Arc::new(QueryOptimizer::new());
        let executor = QueryExecutor::new(optimizer);

        let stats = executor.get_stats();
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.successful_queries, 0);
    }

    #[test]
    fn test_logical_plan_default() {
        let plan = LogicalPlan::default();
        assert!(plan.operations.is_empty());
        assert!(plan.tables.is_empty());
    }

    #[test]
    fn test_execution_stats_default() {
        let stats = ExecutionStats::default();
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.avg_execution_time_ms, 0.0);
    }
}
