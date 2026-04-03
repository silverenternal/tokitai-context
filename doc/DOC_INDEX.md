# Tokitai-Context 文档索引

**最后更新**: 2026-04-03  
**文档总数**: 58  
**总行数**: ~25,000

---

## 🌟 核心文档 (必读)

| 文档 | 描述 | 行数 | 推荐阅读 |
|------|------|------|----------|
| [README.md](../README.md) | 项目主文档 | ~400 | ⭐⭐⭐ |
| [doc/README.md](README.md) | 文档中心导航 | ~400 | ⭐⭐⭐ |
| [doc/ARCHITECTURE.md](ARCHITECTURE.md) | 系统架构 | ~800 | ⭐⭐⭐ |
| [doc/QUICKSTART.md](QUICKSTART.md) | 快速开始 | ~300 | ⭐⭐⭐ |
| [doc/PAPER_CONTRIBUTIONS.md](PAPER_CONTRIBUTIONS.md) | 论文贡献点 | ~1200 | ⭐⭐⭐ |
| [doc/PERFORMANCE_REPORT.md](PERFORMANCE_REPORT.md) | 性能报告 | ~800 | ⭐⭐⭐ |
| [doc/IMPLEMENTATION_SUMMARY_v2.md](IMPLEMENTATION_SUMMARY_v2.md) | 实现总结 | ~1000 | ⭐⭐⭐ |

---

## 📚 文档分类

### 架构与设计 (6 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | 完整系统架构和数据流 | ✅ 新增 |
| [PARALLEL_CONTEXT_IMPLEMENTATION.md](PARALLEL_CONTEXT_IMPLEMENTATION.md) | 平行上下文实现报告 | ✅ |
| [CONTEXT_STORAGE.md](CONTEXT_STORAGE.md) | 上下文存储机制 | ✅ |
| [MECHANISMS.md](MECHANISMS.md) | 核心算法和机制 | ✅ |
| [IMPLEMENTATION_REPORT.md](IMPLEMENTATION_REPORT.md) | 实现细节报告 | ✅ |
| [IMPLEMENTATION_SUMMARY_v2.md](IMPLEMENTATION_SUMMARY_v2.md) | 实现总结 (v2) | ✅ 新增 |

### 论文与性能 (3 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [PAPER_CONTRIBUTIONS.md](PAPER_CONTRIBUTIONS.md) | 论文贡献点和实验设计 | ✅ 新增 |
| [PERFORMANCE_REPORT.md](PERFORMANCE_REPORT.md) | 完整性能基准测试 | ✅ 新增 |
| [BENCHMARK_REPORT.md](BENCHMARK_REPORT.md) | 基准测试结果 | ✅ |

### 存储引擎 (8 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [FILEKV_OPTIMIZATION_REPORT.md](FILEKV_OPTIMIZATION_REPORT.md) | LSM-Tree 优化报告 | ✅ |
| [FILEKV_OPTIMIZATION_PLAN.json](FILEKV_OPTIMIZATION_PLAN.json) | 优化计划 | ✅ |
| [BLOOM_FILTER_PERSISTENCE.md](BLOOM_FILTER_PERSISTENCE.md) | Bloom Filter 持久化 | ✅ |
| [BLOOM_FILTER_MEMORY_OPTIMIZATION.md](BLOOM_FILTER_MEMORY_OPTIMIZATION.md) | Bloom Filter 内存优化 | ✅ |
| [COMPACTION_IMPLEMENTATION.md](COMPACTION_IMPLEMENTATION.md) | Compaction 实现 | ✅ |
| [ADAPTIVE_PREALLOCATION.md](ADAPTIVE_PREALLOCATION.md) | 自适应预分配 | ✅ |
| [KV_FILE_DESIGN.md](KV_FILE_DESIGN.md) | KV 文件设计 | ✅ |
| [OPTIMIZATION_SUMMARY.md](OPTIMIZATION_SUMMARY.md) | 优化总结 | ✅ |

### 崩溃恢复 (4 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [P1_005_CRASH_RECOVERY_TESTS.md](P1_005_CRASH_RECOVERY_TESTS.md) | 崩溃恢复测试 | ✅ |
| [P2_015_CRASH_RECOVERY.md](P2_015_CRASH_RECOVERY.md) | 崩溃恢复框架 | ✅ |
| [CONSISTENCY_CHECK.md](CONSISTENCY_CHECK.md) | 数据一致性校验 | ✅ |
| [CONCURRENCY.md](CONCURRENCY.md) | 并发模型 | ✅ |

### AI 增强 (3 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [P3_008_AI_AUTO_TUNING.md](P3_008_AI_AUTO_TUNING.md) | AI 自动调参 | ✅ |
| [CACHE_WARMING.md](CACHE_WARMING.md) | 缓存预热 | ✅ |
| [purpose_inference](../src/purpose_inference.rs) | 目的推断 (代码) | ✅ |

### 优化与修复 (12 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [P0_P1_FIXES_SUMMARY_v2.md](P0_P1_FIXES_SUMMARY_v2.md) | P0/P1 修复总结 | ✅ |
| [P0_P1_FIXES_SUMMARY.md](P0_P1_FIXES_SUMMARY.md) | P0/P1 修复 (旧) | 📜 |
| [P1_001_PERFORMANCE_FIX.md](P1_001_PERFORMANCE_FIX.md) | 性能修复报告 | ✅ |
| [P1_013_WAL_ROTATION.md](P1_013_WAL_ROTATION.md) | WAL 轮转实现 | ✅ |
| [P2_COMPLETION_SUMMARY.md](P2_COMPLETION_SUMMARY.md) | P2 特性完成总结 | ✅ |
| [P2_006_LOCK_FREE_MEMTABLE.md](P2_006_LOCK_FREE_MEMTABLE.md) | 无锁 MemTable | ✅ |
| [P2_007_BACKPRESSURE.md](P2_007_BACKPRESSURE.md) | 背压机制 | ✅ |
| [P2_009_INCREMENTAL_CHECKPOINT.md](P2_009_INCREMENTAL_CHECKPOINT.md) | 增量 checkpoint | ✅ |
| [P2_010_MVCC.md](P2_010_MVCC.md) | MVCC 实现 | ✅ |
| [P2_013_AUDIT_LOGGING.md](P2_013_AUDIT_LOGGING.md) | 审计日志 | ✅ |
| [P2_014_COMPRESSION_DICTIONARY.md](P2_014_COMPRESSION_DICTIONARY.md) | 压缩字典 | ✅ |
| [P2_016_PROMETHEUS_METRICS.md](P2_016_PROMETHEUS_METRICS.md) | Prometheus 指标 | ✅ |

### 高级特性 (8 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [P3_001_ASYNC_IO.md](P3_001_ASYNC_IO.md) | 异步 I/O 规划 | ✅ |
| [P3_002_SIMD_CHECKSUMS.md](P3_002_SIMD_CHECKSUMS.md) | SIMD 校验和 | ✅ |
| [P3_003_PITR.md](P3_003_PITR.md) | 时间点恢复 | ✅ |
| [P3_004_DISTRIBUTED_COORDINATION.md](P3_004_DISTRIBUTED_COORDINATION.md) | 分布式协调 | ✅ |
| [P3_005_COLUMN_FAMILY.md](P3_005_COLUMN_FAMILY.md) | 列族支持 | ✅ |
| [P3_006_FUSE_FILESYSTEM.md](P3_006_FUSE_FILESYSTEM.md) | FUSE 文件系统 | ✅ |
| [P3_007_QUERY_OPTIMIZER.md](P3_007_QUERY_OPTIMIZER.md) | 查询优化器 | ✅ |
| [P3_008_AI_AUTO_TUNING.md](P3_008_AI_AUTO_TUNING.md) | AI 自动调参 | ✅ |

### 质量与规范 (5 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [UNSAFE_BLOCKS_AUDIT.md](UNSAFE_BLOCKS_AUDIT.md) | unsafe 代码审查 | ✅ |
| [ERROR_HANDLING.md](ERROR_HANDLING.md) | 错误处理规范 | ✅ |
| [TRACING_CLASSIFICATION.md](TRACING_CLASSIFICATION.md) | tracing 日志分类 | ✅ |
| [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) | 迁移指南 | ✅ |
| [USER_GUIDE.md](USER_GUIDE.md) | 用户指南 | ✅ |

### 其他文档 (9 个)

| 文档 | 描述 | 状态 |
|------|------|------|
| [DOCUMENTATION_SUMMARY.md](DOCUMENTATION_SUMMARY.md) | 文档体系总结 | ✅ |
| [DOC_INDEX.md](DOC_INDEX.md) | 旧文档索引 | 📜 |
| [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) | 实现总结 (旧) | 📜 |
| [IMPROVEMENTS.md](IMPROVEMENTS.md) | 改进记录 | ✅ |
| [OPTIMIZATION_IMPLEMENTATION_REPORT.md](OPTIMIZATION_IMPLEMENTATION_REPORT.md) | 优化实现报告 | ✅ |
| [OPTIMIZATION_PLAN_STATUS.md](OPTIMIZATION_PLAN_STATUS.md) | 优化计划状态 | ✅ |
| [PARALLEL_CONTEXT_OPTIMIZATIONS.md](PARALLEL_CONTEXT_OPTIMIZATIONS.md) | 平行上下文优化 | ✅ |
| [SURVEY_REPORT.md](SURVEY_REPORT.md) | 调研报告 | ✅ |
| [QUICKSTART.md](QUICKSTART.md) | 快速开始 | ✅ |

---

## 📊 文档统计

### 按类别

| 类别 | 文档数 | 总行数 |
|------|--------|--------|
| 核心文档 | 7 | ~5,000 |
| 架构设计 | 6 | ~4,000 |
| 论文性能 | 3 | ~2,500 |
| 存储引擎 | 8 | ~5,000 |
| 崩溃恢复 | 4 | ~2,500 |
| AI 增强 | 3 | ~2,000 |
| 优化修复 | 12 | ~6,000 |
| 高级特性 | 8 | ~4,000 |
| 质量规范 | 5 | ~2,000 |
| 其他 | 9 | ~3,000 |
| **总计** | **58** | **~25,000** |

### 按状态

| 状态 | 文档数 | 百分比 |
|------|--------|--------|
| ✅ 完成 | 55 | 95% |
| 📜 归档 | 3 | 5% |
| ❌ 未完成 | 0 | 0% |

---

## 🔍 快速查找

### 我想...

| 需求 | 推荐文档 |
|------|----------|
| 快速上手 | [QUICKSTART.md](QUICKSTART.md) |
| 了解架构 | [ARCHITECTURE.md](ARCHITECTURE.md) |
| 准备发论文 | [PAPER_CONTRIBUTIONS.md](PAPER_CONTRIBUTIONS.md) |
| 查看性能 | [PERFORMANCE_REPORT.md](PERFORMANCE_REPORT.md) |
| 学习 FileKV | [FILEKV_OPTIMIZATION_REPORT.md](FILEKV_OPTIMIZATION_REPORT.md) |
| 了解崩溃恢复 | [P1_005_CRASH_RECOVERY_TESTS.md](P1_005_CRASH_RECOVERY_TESTS.md) |
| 修复 Bug | [P0_P1_FIXES_SUMMARY_v2.md](P0_P1_FIXES_SUMMARY_v2.md) |
| 配置监控 | [P2_016_PROMETHEUS_METRICS.md](P2_016_PROMETHEUS_METRICS.md) |

---

## 📝 更新日志

### 2026-04-03 (v2.0)

**新增文档**:
- ✅ [ARCHITECTURE.md](ARCHITECTURE.md) - 系统架构
- ✅ [PAPER_CONTRIBUTIONS.md](PAPER_CONTRIBUTIONS.md) - 论文贡献点
- ✅ [PERFORMANCE_REPORT.md](PERFORMANCE_REPORT.md) - 性能报告
- ✅ [IMPLEMENTATION_SUMMARY_v2.md](IMPLEMENTATION_SUMMARY_v2.md) - 实现总结

**更新文档**:
- ✅ [README.md](../README.md) - 项目主文档
- ✅ [doc/README.md](README.md) - 文档中心

### 2026-04-02 (v1.0)

- 初始文档体系建立
- 23 个核心文档完成

---

## 🔗 相关链接

- **项目 README**: [../README.md](../README.md)
- **GitHub**: https://github.com/silverenternal/tokitai
- **Crate**: https://crates.io/crates/tokitai-context (待发布)

---

**最后更新**: 2026-04-03  
**维护者**: Tokitai Team  
**许可证**: MIT OR Apache-2.0
