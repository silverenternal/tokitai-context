# Tokitai-Context 实验设计文档

**版本**: 1.0  
**日期**: 2026-04-04  
**目标会议**: ICSE 2027 / AAAI 2027 / VLDB 2027

---

## 📋 目录

1. [实验体系概述](#实验体系概述)
2. [RQ1: 性能实验](#rq1-性能实验)
3. [RQ2: 功能有效性实验](#rq2-功能有效性实验)
4. [RQ3: 用户研究实验](#rq3-用户研究实验)
5. [开源数据集](#开源数据集)
6. [实验代码框架](#实验代码框架)
7. [预期结果展示](#预期结果展示)
8. [实验优先级与时间估算](#实验优先级与时间估算)

---

## 实验体系概述

### 三类实验（ICSE/AAAI 标准要求）

```
┌─────────────────────────────────────────────────────────┐
│                    实验体系                              │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  RQ1: 性能 (Performance)                                │
│  └─ FileKV 比传统方案快多少？                           │
│                                                         │
│  RQ2: 功能有效性 (Effectiveness)                        │
│  └─ Git 风格分支真的有用吗？                            │
│                                                         │
│  RQ3: 用户研究 (User Study)                             │
│  └─ 开发者用起来怎么样？                                │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 研究问题定义

| 研究问题 | 描述 | 评估维度 |
|---------|------|---------|
| **RQ1** | FileKV 存储引擎的性能表现如何？ | 延迟、吞吐量、存储效率 |
| **RQ2** | Git 风格分支管理是否有效？ | 分支创建、merge 成功率、冲突解决 |
| **RQ3** | 开发者使用体验如何？ | 认知负荷、可用性、推荐意愿 |

---

## RQ1: 性能实验

### 1.1 对比基线

| 系统 | 类型 | 对比维度 | 配置 |
|------|------|---------|------|
| **FileKV (Ours)** | 纯文件 LSM-Tree | 所有维度 | 默认配置 |
| **LangChain Memory** | 内存/向量存储 | 写入/读取延迟 | ConversationBufferMemory |
| **ChromaDB** | 向量数据库 | 语义搜索性能 | 默认配置 |
| **SQLite** | 传统数据库 | 存储效率、崩溃恢复 | WAL 模式 |
| **RocksDB** | LSM-Tree | 写入吞吐量 | 默认配置 |

### 1.2 性能指标

#### 写入性能

```python
write_metrics = {
    "单次写入 (64B)": "ns",
    "单次写入 (256B)": "ns",
    "单次写入 (1KB)": "ns",
    "单次写入 (4KB)": "ns",
    "单次写入 (16KB)": "ns",
    "批量写入 (10 项)": "µs/item",
    "批量写入 (100 项)": "µs/item",
    "批量写入 (1000 项)": "µs/item",
}
```

#### 读取性能

```python
read_metrics = {
    "热读取 (cache hit)": "ns",
    "温读取 (index hit)": "µs",
    "冷读取 (disk access)": "ms",
    "语义搜索 (top-10)": "ms",
    "范围查询 (100 项)": "ms",
}
```

#### 分支操作性能

```python
branch_metrics = {
    "Fork 创建": "ms",
    "Checkout": "ms",
    "Merge (无冲突)": "ms",
    "Merge (有冲突)": "ms",
    "时间旅行": "ms",
}
```

#### 存储效率

```python
storage_metrics = {
    "去重率": "%",
    "压缩比": "x",
    "COW 存储节省": "%",
    "空间放大系数": "x",
}
```

### 1.3 实验设计

#### 实验 1.1: 写入性能对比

**目标**: 对比各系统的写入延迟

**方法**:
```python
async def benchmark_write_performance():
    systems = {
        "FileKV": FileKV.open(config),
        "LangChain": ConversationBufferMemory(),
        "SQLite": sqlite3.connect(":memory:"),
        "RocksDB": rocksdb.DB("test.db"),
    }
    
    sizes = [64, 256, 1024, 4096, 16384]  # bytes
    batch_sizes = [1, 10, 100, 1000]
    
    results = {}
    for size in sizes:
        for batch in batch_sizes:
            for name, system in systems.items():
                latency = await measure_write(system, size, batch)
                results[(name, size, batch)] = latency
    
    return results
```

**预期结果**:
- FileKV 单次写入优于 LangChain 10-50x
- FileKV 批量写入优于 RocksDB 1.5-3x
- FileKV 接近 SQLite 性能

---

#### 实验 1.2: 读取性能对比

**目标**: 对比各系统的读取延迟

**方法**:
```python
async def benchmark_read_performance():
    # 预写入 10000 条数据
    for i in range(10000):
        await system.put(f"key_{i}", b"x" * 1024)
    
    # 测试热读取 (100% 命中缓存)
    hot_latencies = []
    for i in range(1000):
        lat = await measure_read(system, f"key_{i}")
        hot_latencies.append(lat)
    
    # 测试冷读取 (0% 命中缓存)
    cold_latencies = []
    for i in range(10000, 11000):
        await system.put(f"key_{i}", b"x" * 1024)
        lat = await measure_read(system, f"key_{i}")
        cold_latencies.append(lat)
    
    return {
        "hot_p99": percentile(hot_latencies, 99),
        "cold_p99": percentile(cold_latencies, 99),
    }
```

**预期结果**:
- FileKV 热读取 <10µs (BlockCache 命中)
- FileKV 冷读取 <1ms (SSD)
- 优于 LangChain 5-10x

---

#### 实验 1.3: 分支操作性能

**目标**: 测量 COW fork 和 merge 的性能

**方法**:
```python
async def benchmark_branch_operations():
    # Fork 创建
    fork_latencies = []
    for i in range(100):
        start = time.perf_counter_ms()
        system.create_branch(f"branch_{i}", "main")
        end = time.perf_counter_ms()
        fork_latencies.append(end - start)
    
    # Merge 操作
    merge_latencies = []
    for i in range(50):
        start = time.perf_counter_ms()
        result = system.merge(f"branch_{i}", "main", "SelectiveMerge")
        end = time.perf_counter_ms()
        merge_latencies.append(end - start)
    
    return {
        "fork_avg_ms": statistics.mean(fork_latencies),
        "merge_avg_ms": statistics.mean(merge_latencies),
    }
```

**预期结果**:
- Fork 创建 <10ms (COW)
- Merge (无冲突) <50ms
- Merge (有冲突) <200ms

---

#### 实验 1.4: 存储效率

**目标**: 测量去重率和压缩效果

**方法**:
```python
def benchmark_storage_efficiency():
    # 写入重复数据
    for i in range(1000):
        system.store("session", b"identical_content" * 100, "short-term")
    
    # 计算去重率
    raw_size = 1000 * len(b"identical_content" * 100)
    actual_size = get_disk_usage(system)
    dedup_ratio = raw_size / actual_size
    
    # 写入可压缩数据
    for i in range(1000):
        system.store("session", b"pattern_" * 1000, "short-term")
    
    # 计算压缩比
    compressed_size = get_disk_usage(system)
    compression_ratio = raw_size / compressed_size
    
    return {
        "dedup_ratio": dedup_ratio,
        "compression_ratio": compression_ratio,
    }
```

**预期结果**:
- 去重率 >10x (重复内容)
- 压缩比 >3x (可压缩内容)
- COW 存储节省 >80%

---

## RQ2: 功能有效性实验

### 2.1 分支场景测试

#### 场景 1: 代码重构方案探索

```python
scenario_refactoring = {
    "description": "探索多种代码重构方案",
    "setup": {
        "main": "原始代码",
        "branches": {
            "branch_1": "方案 A: 函数式重构",
            "branch_2": "方案 B: 面向对象重构",
            "branch_3": "方案 C: 混合重构",
        }
    },
    "tasks": [
        "创建 3 个重构方案分支",
        "在各分支中与 AI 讨论方案细节",
        "比较各方案差异",
        "合并最佳方案到 main",
    ],
    "expected": "能成功 merge 最佳方案，无数据丢失",
    "metrics": {
        "branch_creation_time": "<10ms",
        "merge_success_rate": ">80%",
        "conflict_resolution_time": "<30s",
    }
}
```

#### 场景 2: 假设调试

```python
scenario_debugging = {
    "description": "多假设并行验证",
    "setup": {
        "main": "正常对话 + bug 描述",
        "branches": {
            "hypothesis_null": "空指针假设验证",
            "hypothesis_timing": "时序问题假设验证",
            "hypothesis_logic": "逻辑错误假设验证",
        }
    },
    "tasks": [
        "创建 3 个假设分支",
        "在各分支中验证假设",
        "比较验证结果",
        "合并正确假设到 main",
    ],
    "expected": "能找到正确假设并 merge",
    "metrics": {
        "hypothesis_count": 3,
        "verification_time": "<5min",
        "success_rate": ">90%",
    }
}
```

#### 场景 3: 创意写作

```python
scenario_creative = {
    "description": "多结局故事创作",
    "setup": {
        "main": "故事主线",
        "branches": {
            "ending_a": "结局 A: 悲剧",
            "ending_b": "结局 B: 喜剧",
            "ending_c": "结局 C: 开放",
        }
    },
    "tasks": [
        "创建 2-3 个不同结局分支",
        "在各分支中完成故事",
        "比较不同结局",
        "选择最佳结局",
    ],
    "expected": "能比较不同结局并选择",
    "metrics": {
        "branch_comparison_time": "<2min",
        "user_satisfaction": ">4/5",
    }
}
```

---

### 2.2 评估指标

| 指标 | 测量方法 | 预期值 | 实际值 |
|------|---------|--------|--------|
| **分支创建时间** | 100 次 fork 平均 | <10ms | TBD |
| **分支存储开销** | fork 前后磁盘使用 | <20% 增加 | TBD |
| **Merge 成功率** | 无冲突/总 merge | >80% | TBD |
| **冲突解决时间** | AI 辅助 vs 手动 | 减少 60% | TBD |
| **上下文回溯** | 时间旅行到历史状态 | <50ms | TBD |
| **COW 存储节省** | 实际使用/理论最大 | >80% | TBD |

---

### 2.3 Merge 策略对比

```python
merge_strategies = {
    "FastForward": {
        "description": "直接移动指针",
        "适用场景": "线性开发",
        "预期成功率": "90%",
    },
    "SelectiveMerge": {
        "description": "基于重要性选择",
        "适用场景": "默认策略",
        "预期成功率": "80%",
    },
    "AIAssisted": {
        "description": "AI 辅助冲突解决",
        "适用场景": "复杂合并",
        "预期成功率": "85%",
    },
    "Manual": {
        "description": "用户解决所有冲突",
        "适用场景": "关键变更",
        "预期成功率": "100%",
    },
    "Ours": {
        "description": "保留目标版本",
        "适用场景": "保守策略",
        "预期成功率": "100%",
    },
    "Theirs": {
        "description": "保留源版本",
        "适用场景": "实验性",
        "预期成功率": "100%",
    },
}
```

---

## RQ3: 用户研究实验

### 3.1 实验设计

#### 参与者招募

| 属性 | 要求 | 目标人数 |
|------|------|---------|
| **背景** | AI 开发者/研究者 | 15-20 人 |
| **经验** | 1 年以上编程经验 | - |
| **分组** | 实验组 vs 对照组 | 各 8-10 人 |
| **补偿** | $50 礼品卡 | - |

#### 任务设计

```
任务 1: 多方案探索（30 分钟）
  - 用 AI 探索 3 种代码重构方案
  - 比较方案差异
  - 合并最佳方案

任务 2: 假设调试（30 分钟）
  - 给定一个 bug 场景
  - 创建 3 个假设分支
  - 验证并合并正确假设

任务 3: 创意写作（30 分钟）
  - 写一个故事
  - 探索 2 种不同结局
  - 比较并选择
```

---

### 3.2 评估指标

#### 定量指标

| 指标 | 测量方式 | 目标值 |
|------|---------|--------|
| **任务完成时间** | 计时 | 实验组 < 对照组 |
| **任务成功率** | 完成/尝试 | >80% |
| **错误率** | 错误操作/总操作 | <10% |
| **分支使用频率** | 平均每人创建分支数 | >3 |

#### 定性指标

| 指标 | 问卷 | 目标值 |
|------|------|--------|
| **认知负荷** | NASA-TLX | <50/100 |
| **系统可用性** | SUS | >70/100 |
| **推荐意愿** | NPS | >8/10 |
| **满意度** | 5 点 Likert | >4/5 |

---

### 3.3 实验流程

```
1. 欢迎与介绍 (5 分钟)
   - 介绍实验目的
   - 签署知情同意书

2. 培训 (10 分钟)
   - 实验组：Tokitai-Context 使用教程
   - 对照组：LangChain 使用教程

3. 任务执行 (90 分钟)
   - 任务 1: 多方案探索 (30 分钟)
   - 任务 2: 假设调试 (30 分钟)
   - 任务 3: 创意写作 (30 分钟)

4. 问卷填写 (15 分钟)
   - NASA-TLX
   - SUS
   - NPS

5. 访谈 (10 分钟)
   - 定性反馈
   - 改进建议
```

---

## 开源数据集

### 1. AI 对话数据集

| 数据集 | 规模 | 用途 | 链接 |
|--------|------|------|------|
| **ShareGPT** | ~200k 对话 | 真实对话负载 | [sharegpt.com](https://sharegpt.com/) |
| **OpenAI Conversation** | ~14k | 多轮对话 | [HF](https://huggingface.co/datasets/openai/openai_summarize_conversations) |
| **Chatbot Arena** | ~30k | 对话质量评估 | [lmsys.org](https://lmsys.org/blog/2024-03-20-chatbot-arena-leaderboard/) |
| **Alpaca** | 52k | 指令跟随 | [GitHub](https://github.com/tatsu-lab/stanford_alpaca) |
| **Dolly** | 15k | 指令数据 | [HF](https://huggingface.co/datasets/databricks/databricks-dolly-15k) |

### 2. 代码对话数据集

| 数据集 | 规模 | 用途 | 链接 |
|--------|------|------|------|
| **CodeAlpaca** | 20k | 代码指令 | [HF](https://huggingface.co/datasets/sahil2801/CodeAlpaca-20k) |
| **CodeFeedback** | 100k | 代码对话 | [HF](https://huggingface.co/datasets/m-a-p/CodeFeedback-Filtered-Instruction) |
| **StackExchange** | ~1M | 技术问答 | [Archive](https://archive.org/details/stackexchange) |

### 3. 多轮对话数据集

| 数据集 | 规模 | 特点 | 链接 |
|--------|------|------|------|
| **MultiWOZ** | 10k | 多轮任务型对话 | [HF](https://huggingface.co/datasets/multi_woz_v22) |
| **ConvAI2** | 25k | 个性化对话 | [HF](https://huggingface.co/datasets/conv_ai_2) |
| **DailyDialog** | 13k | 日常对话 | [HF](https://huggingface.co/datasets/daily_dialog) |

---

### 数据集使用建议

**首选数据集**:
1. **ShareGPT** - 真实对话负载，规模大
2. **Alpaca** - 指令跟随场景，格式标准
3. **CodeAlpaca** - 代码场景，贴近目标用户

**数据预处理**:
```python
def preprocess_sharegpt(raw_data):
    """将 ShareGPT 数据转换为 Tokitai-Context 格式"""
    conversations = []
    for item in raw_data:
        conversation = {
            "session_id": item["id"],
            "messages": [
                {"role": msg["from"], "content": msg["value"]}
                for msg in item["conversations"]
            ],
        }
        conversations.append(conversation)
    return conversations
```

---

## 实验代码框架

### 性能 Benchmark

```python
# benchmarks/performance_test.py

import asyncio
import time
from typing import Dict, List
import statistics

class PerformanceBenchmark:
    def __init__(self):
        self.results = {}
    
    async def benchmark_write(self, system, size: int, batch: int) -> float:
        """测量写入延迟"""
        data = b"x" * size
        
        start = time.perf_counter_ns()
        for i in range(batch):
            await system.put(f"key_{i}", data)
        end = time.perf_counter_ns()
        
        return (end - start) / batch  # ns per write
    
    async def benchmark_read(self, system, key: str) -> float:
        """测量读取延迟"""
        start = time.perf_counter_ns()
        await system.get(key)
        end = time.perf_counter_ns()
        return end - start
    
    async def benchmark_fork(self, system, num_forks: int) -> float:
        """测量 fork 创建时间"""
        start = time.perf_counter_ms()
        for i in range(num_forks):
            system.create_branch(f"branch_{i}", "main")
        end = time.perf_counter_ms()
        
        return (end - start) / num_forks  # ms per fork
    
    async def benchmark_merge(self, system, strategy: str) -> Dict:
        """测量 merge 性能"""
        # 创建冲突场景
        system.store("main", b"original", "short-term")
        system.store("branch1", b"modification_a", "short-term")
        system.store("branch2", b"modification_b", "short-term")
        
        start = time.perf_counter_ms()
        result = system.merge("branch1", "branch2", strategy)
        end = time.perf_counter_ms()
        
        return {
            "latency_ms": end - start,
            "success": result.success,
            "conflicts": result.conflict_count
        }
    
    async def run_all(self):
        """运行完整 benchmark"""
        systems = {
            "FileKV": FileKV.open(test_config),
            "LangChain": ConversationBufferMemory(),
            "SQLite": SQLiteBackend(":memory:"),
        }
        
        for name, system in systems.items():
            print(f"Benchmarking {name}...")
            
            # 写入性能
            for size in [64, 256, 1024, 4096]:
                latency = await self.benchmark_write(system, size, 100)
                self.results[(name, "write", size)] = latency
            
            # 分支性能
            fork_latency = await self.benchmark_fork(system, 100)
            self.results[(name, "fork", "avg")] = fork_latency
        
        return self.results
```

---

### 功能测试框架

```python
# tests/functional_test.py

import pytest
from tokitai_context import Context

class TestBranchOperations:
    @pytest.fixture
    def context(self):
        ctx = Context.open(":memory:")
        yield ctx
        ctx.close()
    
    def test_fork_creation(self, context):
        """测试 fork 创建"""
        start = time.perf_counter_ms()
        context.create_branch("feature-1", "main")
        end = time.perf_counter_ms()
        
        assert (end - start) < 10  # <10ms
        assert context.get_branch("feature-1") is not None
    
    def test_cow_storage_saving(self, context):
        """测试 COW 存储节省"""
        # 在 main 写入数据
        for i in range(100):
            context.store("main", b"identical_content" * 100, "short-term")
        
        main_size = context.get_disk_usage()
        
        # 创建 fork
        context.create_branch("feature-1", "main")
        
        feature_size = context.get_disk_usage()
        
        # COW 应该只增加少量开销
        size_increase = (feature_size - main_size) / main_size
        assert size_increase < 0.2  # <20% 增加
    
    def test_merge_success_rate(self, context):
        """测试 merge 成功率"""
        # 创建分支
        context.create_branch("feature-1", "main")
        context.create_branch("feature-2", "main")
        
        # 写入不冲突的数据
        context.store("feature-1", b"content_1", "short-term")
        context.store("feature-2", b"content_2", "short-term")
        
        # 合并
        result = context.merge("feature-1", "main", "SelectiveMerge")
        
        assert result.success
        assert result.conflict_count == 0
```

---

## 预期结果展示

### Figure 1: 写入性能对比

```
写入延迟 (ns/item, log 尺度)
│
│     ████
│     ████              ████
│     ████  ████        ████
│     ████  ████  ████  ████
│     ████  ████  ████  ████
├─────────────────────────────
      FileKV  LangChain  SQLite  RocksDB
      (64B)   (64B)      (64B)   (64B)
  
Figure 1: FileKV 写入性能优于对比系统 10-50x
```

### Figure 2: 分支创建效率

```
Fork 创建时间 (ms)
│
│   ████
│   ████
│   ████
│   ████              ████
│   ████              ████
│   ████  ████        ████
│   ████  ████  ████  ████
├─────────────────────────────
      COW     复制     Git     传统
   (Ours)
  
Figure 2: COW fork 比直接复制快 300x
```

### Figure 3: 存储效率对比

```
存储开销 (归一化，越小越好)
│
│   ████
│   ████  ████
│   ████  ████  ████
│   ████  ████  ████  ████
│   ████  ████  ████  ████
├─────────────────────────────
   FileKV  LangChain  SQLite  RocksDB
  (COW)   (无去重)   (无去重) (无去重)

Figure 3: COW 去重节省 80%+ 存储空间
```

---

## 实验优先级与时间估算

### 优先级排序

| 实验 | 优先级 | 时间估算 | 必要性 | 依赖 |
|------|--------|---------|--------|------|
| **写入/读取性能对比** | ⭐⭐⭐⭐⭐ | 1-2 天 | 必须 | 无 |
| **分支创建效率** | ⭐⭐⭐⭐⭐ | 1 天 | 必须 | 无 |
| **Merge 成功率** | ⭐⭐⭐⭐ | 1 天 | 重要 | 无 |
| **存储效率（去重）** | ⭐⭐⭐⭐ | 1 天 | 重要 | 无 |
| **用户研究** | ⭐⭐⭐ | 1-2 周 | 加分 | 招募 |
| **消融实验** | ⭐⭐⭐ | 2-3 天 | 加分 | 无 |
| **可扩展性实验** | ⭐⭐ | 1-2 天 | 可选 | 大数据集 |

---

### 最小可行实验 (MVE)

如果时间有限，至少完成以下实验：

```
✅ 性能对比：FileKV vs LangChain vs SQLite（写入/读取延迟）
✅ 分支效率：Fork 创建时间、存储开销
✅ Merge 效果：成功率、冲突解决时间
✅ 存储效率：去重率、压缩比
```

**时间估算**: 1 周

---

### 完整实验 (顶会标准)

```
✅ 性能对比（4 个系统 × 5 种负载）
✅ 分支场景测试（3 个场景）
✅ 用户研究（15-20 人）
✅ 消融实验（EWMA、COW、Write Coalescing 单独评估）
✅ 可扩展性实验（100 万条目）
```

**时间估算**: 2-3 周

---

## 实验执行清单

### 阶段 1: 性能实验 (1 周)

- [ ] 搭建 benchmark 环境
- [ ] 准备对比系统 (LangChain, SQLite, RocksDB)
- [ ] 运行写入性能实验
- [ ] 运行读取性能实验
- [ ] 运行分支操作实验
- [ ] 运行存储效率实验
- [ ] 整理数据，绘制图表

### 阶段 2: 功能实验 (1 周)

- [ ] 设计分支场景测试
- [ ] 准备测试数据集 (ShareGPT, Alpaca)
- [ ] 运行场景 1: 代码重构
- [ ] 运行场景 2: 假设调试
- [ ] 运行场景 3: 创意写作
- [ ] 整理结果，绘制图表

### 阶段 3: 用户研究 (2 周，可选)

- [ ] 设计实验流程
- [ ] 招募参与者 (15-20 人)
- [ ] 执行实验
- [ ] 收集问卷数据
- [ ] 统计分析
- [ ] 整理结果

### 阶段 4: 论文撰写 (2-3 周)

- [ ] 撰写 Introduction
- [ ] 撰写 System Design
- [ ] 撰写 Evaluation
- [ ] 撰写 Related Work
- [ ] 撰写 Conclusion
- [ ] 准备 submission

---

## 附录：实验配置

### 硬件配置

```
CPU: Intel Core i9-13900K / AMD Ryzen 9 7950X
内存：32GB DDR5
存储：1TB NVMe SSD (Samsung 980 Pro)
操作系统：Ubuntu 22.04 LTS
```

### 软件配置

```
Rust: 1.75+
Python: 3.10+
LangChain: 0.1+
SQLite: 3.40+
RocksDB: 8.0+
```

### 参数配置

```toml
# FileKV 配置
[FileKV]
memtable_flush_threshold_bytes = 4194304  # 4MB
block_cache_size_bytes = 67108864  # 64MB
segment_preallocate_size = 16777216  # 16MB
enable_wal = true
enable_bloom = true
enable_background_flush = true
```

---

**文档结束**

---

## 参考文献

1. LangChain Documentation. https://python.langchain.com/
2. RocksDB Documentation. https://rocksdb.org/
3. ShareGPT Dataset. https://sharegpt.com/
4. Alpaca Dataset. https://github.com/tatsu-lab/stanford_alpaca
5. ICSE 2027 Submission Guidelines. https://icse2027.org/
