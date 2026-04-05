# 阿里云 Coding Plan 配置指南

**最后更新**: 2026-04-04  
**版本**: 1.0.0

---

## 📋 概述

阿里云百炼（Model Studio）Coding Plan 是阿里云提供的 AI 编码套餐服务，支持通义千问系列模型（Qwen），为 AI 编码场景提供优化。

---

## 🎯 主要特性

- **固定月费**：避免按量付费的账单波动
- **月度请求额度**：提供固定的每月请求配额
- **支持的模型**：qwen-plus, qwen-max, qwen-turbo, qwen-long 等
- **OpenAI 兼容 API**：使用 OpenAI 兼容的 API 格式

---

## ⚠️ 重要提醒

**必须使用 Coding Plan 专用的 Base URL 和 API Key**

如果使用普通 Base URL 或 API Key，将按**按量付费**计费，而非从套餐额度扣除。

---

## 🔑 获取 API Key

### 步骤 1：访问阿里云百炼控制台

打开 https://bailian.console.aliyun.com

### 步骤 2：订阅 Coding Plan

1. 登录阿里云账号
2. 进入百炼控制台
3. 找到 Coding Plan（AI 编码套餐）
4. 订阅套餐计划

### 步骤 3：获取专用 API Key

1. 在 Coding Plan 页面找到 API Key 管理
2. 创建或复制你的 Coding Plan 专用 API Key
3. 记录专用的 Base URL

---

## 🔧 配置方式

### 方式一：使用 .env 文件（推荐）

```bash
# 复制模板
cp .env.example .env

# 编辑 .env 文件
```

**`.env` 文件内容**：
```bash
# 阿里云百炼 AI 编码套餐
ALIYUN_CODING_PLAN_API_KEY="your-coding-plan-api-key"
# 或使用替代变量名
DASHSCOPE_API_KEY="your-dashscope-api-key"
```

### 方式二：使用环境变量

```bash
# 设置 API Key
export ALIYUN_CODING_PLAN_API_KEY="your-coding-plan-api-key"

# 或使用替代变量名
export DASHSCOPE_API_KEY="your-dashscope-api-key"
```

---

## 💻 Rust 代码使用

### 基础使用

```rust
use tokitai_context::ai::clients::AliyunCodingPlanClient;
use tokitai_context::ai::client::LLMClient;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 从 .env 文件或环境变量加载配置
    let client = AliyunCodingPlanClient::from_env();
    let llm: Arc<dyn LLMClient> = Arc::new(client);

    // 使用客户端
    let response = llm.chat("Hello").await?;
    println!("{}", response);

    Ok(())
}
```

### 自定义配置

```rust
use tokitai_context::ai::clients::{AliyunCodingPlanClient, AliyunCodingPlanConfig};

// 自定义模型和配置
let config = AliyunCodingPlanConfig::default()
    .with_model("qwen-plus")
    .with_api_key("your-coding-plan-api-key")
    .with_base_url("https://dashscope.aliyuncs.com/compatible-mode/v1");

let client = AliyunCodingPlanClient::with_config(config);
```

### 切换模型

```rust
// 使用 qwen-max（更高性能）
let config = AliyunCodingPlanConfig::default()
    .with_model("qwen-max");

// 使用 qwen-turbo（更快速度）
let config = AliyunCodingPlanConfig::default()
    .with_model("qwen-turbo");
```

---

## 📊 支持的模型

| 模型 | 描述 | 适用场景 |
|------|------|---------|
| `qwen-plus` | 平衡性能和成本 | 通用编码任务 |
| `qwen-max` | 最高性能 | 复杂代码分析、重构 |
| `qwen-turbo` | 最快速度 | 简单代码生成、快速迭代 |
| `qwen-long` | 长上下文支持 | 大文件分析、长文档处理 |

---

## 🧪 运行示例

```bash
# 配置 API Key
export ALIYUN_CODING_PLAN_API_KEY="your-api-key"

# 运行示例程序
cargo run --features ai --example aliyun_coding_plan
```

---

## 📈 监控使用量

在阿里云百炼控制台的 Coding Plan 页面可以查看：
- 剩余请求额度
- 本月已用请求数
- 请求消耗统计

---

## 🔒 安全最佳实践

1. **不要硬编码 API Key**：始终使用环境变量或 `.env` 文件
2. **保护 `.env` 文件**：确保 `.env` 在 `.gitignore` 中
3. **定期轮换密钥**：定期更新 API Key
4. **最小权限原则**：仅授予必要的权限

---

## 🐛 常见问题

### Q: 如何确认使用的是 Coding Plan 专用 API Key？

A: 在阿里云百炼控制台的 Coding Plan 页面获取的 API Key 即为专用 Key。

### Q: 为什么会按按量付费计费？

A: 可能原因：
1. 使用了普通 API Key 而非 Coding Plan 专用 Key
2. 使用了错误的 Base URL
3. Coding Plan 套餐额度已用尽

### Q: 如何检查 API Key 是否有效？

A: 运行示例程序，如果返回正常响应则 API Key 有效：
```bash
cargo run --features ai --example aliyun_coding_plan
```

### Q: 支持哪些区域？

A: 阿里云百炼支持中国大陆区域，具体可用区请参考控制台。

---

## 📚 相关资源

- [阿里云百炼控制台](https://bailian.console.aliyun.com)
- [通义千问文档](https://help.aliyun.com/zh/model-studio/)
- [Tokitai-Context AI Features Guide](AI_FEATURES_GUIDE.md)
- [Tokitai-Context Quick Start](QUICKSTART.md)

---

## 💰 计费说明

- **固定月费**：Coding Plan 采用固定月费制
- **额度限制**：每月有固定的请求额度
- **超额处理**：超出额度后可能需要升级套餐或按量付费

具体价格请参考阿里云官方定价页面。

---

**最后更新**: 2026-04-04  
**维护者**: Tokitai Team
