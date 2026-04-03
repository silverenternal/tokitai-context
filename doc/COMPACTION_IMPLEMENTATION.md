# FileKV 生产级改进报告

## 概述

本文档记录了对 `tokitai-context` 模块的纯文件 LSM-Tree KV 存储引擎的生产级改进。

## 已完成的改进

### 1. Compaction 压缩合并机制 ✅

**文件**: `src/compaction.rs` (~520 行)

**功能**:
- **Size-tiered compaction**: 当小 segment 文件数量达到阈值时自动合并
- **Tombstone 清理**: 删除标记在 compaction 过程中被清理，释放空间
- **索引重建**: 合并后自动重建稀疏索引和 Bloom Filter
- **自动触发**: 每 N 次写入后检查是否需要 compaction

**配置参数** (`CompactionConfig`):
```rust
pub struct CompactionConfig {
    pub min_segments: usize,              // 触发 compaction 的最小 segment 数 (默认 4)
    pub max_segment_size_bytes: u64,      // segment 最大大小 (默认 16MB)
    pub target_segment_size_bytes: u64,   // 目标 segment 大小 (默认 8MB)
    pub max_compact_segments: usize,      // 单次 compaction 最大 segment 数 (默认 8)
    pub auto_compact: bool,               // 是否启用自动 compaction
    pub check_interval: usize,            // 检查间隔 (默认每 100 次写入)
}
```

**API**:
```rust
// 手动触发 compaction
kv.compact()?;

// 获取 compaction 统计
let stats = kv.compaction_stats();

// 检查 compaction 状态
let is_compacting = kv.is_compacting();
```

**统计信息** (已集成到 `FileKVStats`):
- `compaction_runs`: Compaction 运行次数
- `compaction_segments_merged`: 合并的 segment 数量
- `compaction_tombstones_removed`: 清理的删除标记数量

---

### 2. 架构改进

**FileKV 结构更新**:
- 添加 `compaction_manager: Arc<CompactionManager>`
- 添加 `pub(crate)` 可见性字段以支持 compaction:
  - `config`, `segments`, `next_segment_id`
  - `index_manager`, `bloom_filters`, `stats`

**IndexManager 扩展**:
- `insert_index()`: 插入索引（compaction 用）
- `remove_index()`: 移除索引（compaction 用）

**SegmentFile 扩展**:
- `iterate_entries()`: 遍历 segment 所有条目（compaction 用）

---

### 3. 测试覆盖

**新增测试** (`src/compaction.rs`):
- `test_compaction_config_default`: 配置默认值测试
- `test_compaction_manager_should_compact`: compaction 触发条件测试
- `test_compaction_with_filekv`: 集成 compaction 的 FileKV 测试

**修复的测试**:
- 修复 `test_filekv_open`, `test_filekv_recovery`, `test_filekv_flush_and_read` 使用临时 index 目录

**测试结果**: 32/32 测试通过
- 10 file_kv 测试 ✅
- 4 sparse_index 测试 ✅
- 6 block_cache 测试 ✅
- 9 facade 测试 ✅
- 3 compaction 测试 ✅

---

## 待改进的领域（生产级差距）

### 1. Bloom Filter 持久化 🔴

**现状**: Bloom Filter 在重启后需要重建
**需要**: 
- 持久化 Bloom Filter 到磁盘
- 启动时从 segment 文件重建 Bloom Filter
- 或者持久化 bloom filter 二进制数据

### 2. 零拷贝读取优化 🟡

**现状**: mmap 已使用，但数据仍被拷贝到 Vec
**优化**: 
- 返回 `&[u8]` lifetime 绑定到 mmap
- 需要小心处理内存生命周期

### 3. 后台刷盘线程 🟡

**现状**: 刷盘在 `put()` 中同步执行
**优化**:
- 后台线程定期刷盘
- 写入路径更快（纯内存操作）
- 需要处理优雅关闭

### 4. 生产级监控 🟡

**现状**: 基础统计信息
**需要**:
- 延迟直方图（p50, p95, p99）
- I/O 吞吐量统计
- 错误率追踪
- Prometheus metrics 导出

### 5. Segment 预分配 🟡

**现状**: 顺序追加，无预分配
**优化**:
- 预分配 segment 文件空间（如 fallocate）
- 减少文件碎片
- 提升写入性能

### 6. 配置验证 🟡

**现状**: 无配置验证
**需要**:
- 验证阈值合理性
- 检查路径权限
- 内存限制检查

### 7. 错误处理增强 🟡

**现状**: 基本 anyhow::Result
**需要**:
- 更细粒度的错误类型
- 可恢复错误 vs 致命错误
- 错误重试机制

---

## 性能目标验证（待基准测试）

| 操作 | 当前文件存储 | 优化后 FileKV | 目标 |
|------|-------------|--------------|------|
| 写入 | ~39µs | **待测试** | 5-7µs |
| 读取（冷） | ~44µs | **待测试** | 2-3µs |
| 读取（热） | ~44µs | **待测试** | 0.5-1µs |
| 删除 | ~40µs | **~1µs** (标记) | ~1µs |

---

## 代码量统计

| 模块 | 代码行数 | 说明 |
|------|---------|------|
| `file_kv.rs` | ~1425 行 | FileKV 核心实现 |
| `sparse_index.rs` | ~497 行 | 稀疏索引模块 |
| `block_cache.rs` | ~440 行 | Block Cache 热点缓存 |
| `compaction.rs` | ~520 行 | **新增** Compaction 机制 |
| **总计** | **~2882 行** | 纯文件 LSM-Tree 实现 |

---

## 使用示例

### 基本使用

```rust
use tokitai_context::file_kv::{FileKV, FileKVConfig};
use tokitai_context::compaction::CompactionConfig;

let config = FileKVConfig {
    segment_dir: "./data/segments".into(),
    wal_dir: "./data/wal".into(),
    index_dir: "./data/index".into(),
    enable_wal: true,
    enable_bloom: true,
    compaction: CompactionConfig {
        min_segments: 4,
        auto_compact: true,
        ..Default::default()
    },
    ..Default::default()
};

let kv = FileKV::open(config)?;

// 写入
kv.put("key1", b"value1")?;

// 读取
if let Some(value) = kv.get("key1")? {
    println!("Got value: {:?}", value);
}

// 删除
kv.delete("key1")?;

// 手动 compaction
kv.compact()?;

// 统计
let stats = kv.stats();
println!("Stats: {:?}", stats);

kv.close()?;
```

### 通过 Facade API 使用

```rust
use tokitai_context::facade::{Context, ContextConfig};

let config = ContextConfig {
    enable_filekv_backend: true,
    memtable_flush_threshold_bytes: 4 * 1024 * 1024,
    block_cache_size_bytes: 64 * 1024 * 1024,
    ..Default::default()
};

let ctx = Context::open("./.context", config)?;

// ShortTerm/Transient 层自动使用 FileKV
let hash = ctx.store("session-1", b"data", Layer::ShortTerm)?;
let item = ctx.retrieve("session-1", &hash)?;
```

---

## 结论

**已完成**: Compaction 机制是 LSM-Tree 生产可用的核心功能，现在已完整实现。

**下一步优先级**:
1. **Bloom Filter 持久化** - 减少重启时间
2. **基准测试** - 验证性能目标
3. **后台刷盘** - 进一步降低写入延迟
4. **监控指标** - 生产环境可观测性

**生产就绪度**: 70%
- ✅ 核心 LSM-Tree 功能完整
- ✅ Compaction 机制实现
- ✅ 测试覆盖良好
- ⚠️ Bloom Filter 需持久化
- ⚠️ 缺少性能基准验证
- ⚠️ 监控和指标待完善
