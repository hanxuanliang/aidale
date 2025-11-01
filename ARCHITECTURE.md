# Aidale 架构指南

## 架构概览

```
┌─────────────────────────────────────────────────────────┐
│                      Application                         │
│                   (Your Code)                            │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                  RuntimeExecutor                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │  High-level API                                   │  │
│  │  - generate_text()                                │  │
│  │  - generate_object()                              │  │
│  │  - Plugin orchestration                           │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────────────┘
                     │
          ┌──────────┴──────────┐
          │                     │
          ▼                     ▼
┌──────────────────┐   ┌────────────────────┐
│  PluginEngine    │   │  JsonOutputStrategy│
│  - Tool calls    │   │  - JsonSchema      │
│  - Lifecycle     │   │  - JsonMode        │
└──────────────────┘   └────────────────────┘
          │                     │
          └──────────┬──────────┘
                     ▼
┌─────────────────────────────────────────────────────────┐
│                    Layers (AOP)                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │  Logging   │→ │   Retry    │→ │  Provider  │        │
│  └────────────┘  └────────────┘  └────────────┘        │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                    Provider                              │
│  - chat_completion()                                     │
│  - stream_chat_completion()                              │
│  - HTTP client only, no business logic                   │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                   AI Service                             │
│  (OpenAI, DeepSeek, Anthropic, etc.)                    │
└─────────────────────────────────────────────────────────┘
```

---

## 核心组件

### 1. Provider 层
**职责**: 纯粹的 HTTP 客户端

```rust
#[async_trait]
pub trait Provider: Send + Sync + Debug + 'static {
    fn info(&self) -> Arc<ProviderInfo>;

    async fn chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError>;

    async fn stream_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<Box<ChatCompletionStream>, AiError>;
}
```

**特点**:
- ✅ 只处理 HTTP 通信
- ✅ 不包含业务逻辑
- ✅ 转换请求/响应格式
- ✅ 错误处理和映射

**实现**:
- `OpenAiProvider` - OpenAI API
- 通过配置支持 DeepSeek 等兼容 API

---

### 2. Runtime 层
**职责**: 请求编排和高级 API

```rust
pub struct RuntimeExecutor {
    provider: BoxedProvider,
    plugin_engine: PluginEngine,
    json_strategy: Box<dyn JsonOutputStrategy>,
}

impl RuntimeExecutor {
    // 高级 API
    pub async fn generate_text(...) -> Result<TextResult, AiError>;
    pub async fn generate_object(...) -> Result<ObjectResult, AiError>;
}
```

**特点**:
- ✅ 提供 `generate_text()` 和 `generate_object()` 高级 API
- ✅ 管理插件生命周期
- ✅ 自动选择和应用策略
- ✅ Builder 模式组合 Layer 和 Plugin

**流程**:
```
1. 接收高级请求 (TextParams/ObjectParams)
2. 执行插件 hooks (transform_params, on_request_start)
3. 选择并应用策略 (JSON 输出策略)
4. 转换为 ChatCompletionRequest
5. 调用 Provider
6. 转换响应为高级类型 (TextResult/ObjectResult)
7. 执行插件 hooks (transform_result, on_request_end)
```

---

### 3. Strategy 层
**职责**: 处理 Provider 特定差异

```rust
pub trait JsonOutputStrategy: Send + Sync {
    fn name(&self) -> &str;
    fn apply(
        &self,
        req: &mut ChatCompletionRequest,
        schema: &serde_json::Value,
    ) -> Result<(), AiError>;
}
```

**实现**:

#### JsonSchemaStrategy (OpenAI)
```rust
// 设置 response_format
req.response_format = Some(ResponseFormat::JsonSchema {
    name: "response".to_string(),
    schema: schema.clone(),
    strict: true,
});
```

#### JsonModeStrategy (DeepSeek)
```rust
// 1. 设置 JsonObject 模式
req.response_format = Some(ResponseFormat::JsonObject);

// 2. 注入 schema 到 system message
let instruction = format!(
    "You must respond with valid JSON that matches this schema:\n{}\n...",
    schema_str
);
req.messages.insert(0, Message::system(instruction));
```

**自动检测**:
```rust
pub fn detect_json_strategy(provider_id: &str) -> Box<dyn JsonOutputStrategy> {
    match provider_id {
        "openai" | "anthropic" | "azure" => Box::new(JsonSchemaStrategy::new()),
        "deepseek" => Box::new(JsonModeStrategy::new()),
        _ => Box::new(JsonModeStrategy::new()),
    }
}
```

---

### 4. Layer 层
**职责**: AOP 横切关注点

```rust
pub trait Layer<P: Provider> {
    type LayeredProvider: Provider;
    fn layer(&self, inner: P) -> Self::LayeredProvider;
}

pub trait LayeredProvider: Sized + Provider {
    type Inner: Provider;
    fn inner(&self) -> &Self::Inner;

    // 可选实现的 hook 方法
    async fn layered_chat_completion(...) -> Result<...>;
    async fn layered_stream_chat_completion(...) -> Result<...>;
}
```

**已实现的 Layers**:

#### LoggingLayer
```rust
// 记录所有请求/响应
let executor = RuntimeExecutor::builder(provider)
    .layer(LoggingLayer::new())
    .finish();
```

#### RetryLayer
```rust
// 自动重试失败的请求
let executor = RuntimeExecutor::builder(provider)
    .layer(RetryLayer::new()
        .with_max_retries(3)
        .with_initial_delay(Duration::from_millis(100)))
    .finish();
```

**特点**:
- ✅ 静态分发（编译时）
- ✅ 可组合（链式调用）
- ✅ 零成本抽象

---

### 5. Plugin 层
**职责**: 运行时业务逻辑扩展

```rust
#[async_trait]
pub trait Plugin: Send + Sync + Debug {
    fn name(&self) -> &str;
    fn phases(&self) -> Vec<PluginPhase>;

    // Hook methods
    async fn on_request_start(&self, ctx: &RequestContext) -> Result<(), AiError>;
    async fn transform_params(&self, params: TextParams, ctx: &RequestContext) -> Result<TextParams, AiError>;
    async fn transform_result(&self, result: TextResult, ctx: &RequestContext) -> Result<TextResult, AiError>;
    // ... 更多 hooks
}
```

**已实现的 Plugins**:
- `ToolUsePlugin` - 工具调用支持

**与 Layer 的区别**:
| 特性 | Layer | Plugin |
|------|-------|--------|
| 分发方式 | 静态（编译时） | 动态（运行时） |
| 类型 | 包装 Provider | 扩展功能 |
| 用途 | 横切关注点 | 业务逻辑 |
| 示例 | 日志、重试、缓存 | 工具调用、RAG |

---

## 数据流

### generate_text() 流程

```
Application
    │
    ├─> RuntimeExecutor::generate_text(model, TextParams)
    │       │
    │       ├─> PluginEngine::resolve_model()
    │       ├─> PluginEngine::transform_params()
    │       ├─> PluginEngine::on_request_start()
    │       │
    │       ├─> 构建 ChatCompletionRequest
    │       │       response_format = ResponseFormat::Text
    │       │
    │       ├─> Layer::chat_completion() [例如 LoggingLayer]
    │       │       │
    │       │       ├─> Layer::chat_completion() [例如 RetryLayer]
    │       │       │       │
    │       │       │       └─> Provider::chat_completion()
    │       │       │               │
    │       │       │               └─> HTTP Request to AI Service
    │       │       │                       │
    │       │       │                       └─> ChatCompletionResponse
    │       │       │
    │       │       └─> 记录日志
    │       │
    │       ├─> 转换为 TextResult
    │       ├─> PluginEngine::transform_result()
    │       └─> PluginEngine::on_request_end()
    │
    └─> TextResult
```

### generate_object() 流程

```
Application
    │
    ├─> RuntimeExecutor::generate_object(model, ObjectParams)
    │       │
    │       ├─> 构建 ChatCompletionRequest
    │       │       messages = ObjectParams.messages
    │       │       temperature = ObjectParams.temperature
    │       │
    │       ├─> JsonOutputStrategy::apply(req, schema)
    │       │       │
    │       │       ├─> [JsonSchemaStrategy] 设置 response_format.json_schema
    │       │       │   OR
    │       │       └─> [JsonModeStrategy] 设置 JsonObject + 注入 schema 到 prompt
    │       │
    │       ├─> Layer::chat_completion()
    │       │       └─> Provider::chat_completion()
    │       │               └─> HTTP Request
    │       │
    │       ├─> 提取 JSON 内容
    │       ├─> 解析为 serde_json::Value
    │       │
    │       └─> ObjectResult { object, usage, model }
    │
    └─> ObjectResult
```

---

## 类型系统

### 请求类型层次

```
ChatCompletionRequest (底层)
    ├─> model: String
    ├─> messages: Vec<Message>
    ├─> temperature: Option<f32>
    ├─> response_format: Option<ResponseFormat>
    └─> ...

ResponseFormat
    ├─> Text
    ├─> JsonObject
    └─> JsonSchema { name, schema, strict }

TextParams (高层)
    ├─> messages: Vec<Message>
    ├─> max_tokens: Option<u32>
    └─> ...

ObjectParams (高层)
    ├─> messages: Vec<Message>
    ├─> schema: serde_json::Value
    └─> ...
```

### 响应类型层次

```
ChatCompletionResponse (底层)
    ├─> id: String
    ├─> model: String
    ├─> choices: Vec<Choice>
    └─> usage: Usage

Choice
    ├─> index: u32
    ├─> message: Message
    └─> finish_reason: FinishReason

TextResult (高层)
    ├─> content: String
    ├─> finish_reason: FinishReason
    ├─> usage: Usage
    └─> model: String

ObjectResult (高层)
    ├─> object: serde_json::Value
    ├─> usage: Usage
    └─> model: String
```

---

## 扩展指南

### 添加新 Provider

```rust
use aidale_core::{Provider, ChatCompletionRequest, ChatCompletionResponse};

#[derive(Debug, Clone)]
pub struct MyProvider {
    client: HttpClient,
    info: Arc<ProviderInfo>,
}

#[async_trait]
impl Provider for MyProvider {
    fn info(&self) -> Arc<ProviderInfo> {
        self.info.clone()
    }

    async fn chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError> {
        // 1. 转换请求格式
        let my_req = convert_request(&req)?;

        // 2. 发送 HTTP 请求
        let my_resp = self.client.post("/chat").json(&my_req).send().await?;

        // 3. 转换响应格式
        let resp = convert_response(my_resp)?;

        Ok(resp)
    }

    async fn stream_chat_completion(...) -> Result<...> {
        // 实现流式响应
    }
}
```

### 添加新 Strategy

```rust
use aidale_core::strategy::JsonOutputStrategy;

#[derive(Debug, Clone)]
pub struct MyCustomStrategy;

impl JsonOutputStrategy for MyCustomStrategy {
    fn name(&self) -> &str {
        "MyCustomStrategy"
    }

    fn apply(
        &self,
        req: &mut ChatCompletionRequest,
        schema: &serde_json::Value,
    ) -> Result<(), AiError> {
        // 修改请求以支持 JSON 输出
        req.response_format = Some(ResponseFormat::JsonObject);

        // 可能需要修改 messages
        let instruction = format!("Return JSON matching: {}", schema);
        req.messages.insert(0, Message::system(instruction));

        Ok(())
    }
}

// 使用
let executor = RuntimeExecutor::builder(provider)
    .json_strategy(Box::new(MyCustomStrategy))
    .finish();
```

### 添加新 Layer

```rust
use aidale_core::layer::{Layer, LayeredProvider};

#[derive(Debug, Clone)]
pub struct MyLayer {
    config: MyConfig,
}

impl<P: Provider> Layer<P> for MyLayer {
    type LayeredProvider = MyLayeredProvider<P>;

    fn layer(&self, inner: P) -> Self::LayeredProvider {
        MyLayeredProvider {
            inner,
            config: self.config.clone(),
        }
    }
}

#[derive(Debug)]
pub struct MyLayeredProvider<P> {
    inner: P,
    config: MyConfig,
}

#[async_trait]
impl<P: Provider> LayeredProvider for MyLayeredProvider<P> {
    type Inner = P;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    async fn layered_chat_completion(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError> {
        // 前置处理
        println!("Before request");

        // 调用内层
        let result = self.inner.chat_completion(req).await;

        // 后置处理
        println!("After request");

        result
    }
}

// 实现 Provider trait
#[async_trait]
impl<P: Provider> Provider for MyLayeredProvider<P> {
    fn info(&self) -> Arc<ProviderInfo> {
        LayeredProvider::layered_info(self)
    }

    async fn chat_completion(...) -> Result<...> {
        LayeredProvider::layered_chat_completion(self, req).await
    }

    async fn stream_chat_completion(...) -> Result<...> {
        LayeredProvider::layered_stream_chat_completion(self, req).await
    }
}
```

### 添加新 Plugin

```rust
use aidale_core::plugin::{Plugin, PluginPhase};

#[derive(Debug)]
pub struct MyPlugin {
    config: MyConfig,
}

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    fn phases(&self) -> Vec<PluginPhase> {
        vec![
            PluginPhase::ResolveModel,
            PluginPhase::TransformParams,
            PluginPhase::OnRequestStart,
            PluginPhase::TransformResult,
            PluginPhase::OnRequestEnd,
        ]
    }

    async fn resolve_model(
        &self,
        model: &str,
        ctx: &RequestContext,
    ) -> Result<String, AiError> {
        // 可以修改模型名称
        Ok(model.to_string())
    }

    async fn transform_params(
        &self,
        mut params: TextParams,
        ctx: &RequestContext,
    ) -> Result<TextParams, AiError> {
        // 修改参数
        params.messages.insert(0, Message::system("Custom system message"));
        Ok(params)
    }

    async fn transform_result(
        &self,
        result: TextResult,
        ctx: &RequestContext,
    ) -> Result<TextResult, AiError> {
        // 处理结果
        Ok(result)
    }
}
```

---

## 最佳实践

### 1. Provider 设计
- ✅ 保持简单，只做 HTTP 通信
- ✅ 不要在 Provider 中添加业务逻辑
- ✅ 使用类型转换函数分离关注点
- ✅ 正确处理和映射错误

### 2. Strategy 使用
- ✅ 为每个 provider 特定差异创建 strategy
- ✅ 保持 strategy 职责单一
- ✅ 提供合理的默认 strategy
- ✅ 允许用户自定义 strategy

### 3. Layer 组合
- ✅ Layer 顺序很重要（Logging → Retry → Provider）
- ✅ 使用静态分发获得零成本抽象
- ✅ 每个 Layer 只做一件事
- ✅ 避免 Layer 之间的耦合

### 4. Plugin 开发
- ✅ 只在必要时使用 Plugin（复杂业务逻辑）
- ✅ 简单的功能优先考虑 Layer
- ✅ 清楚地定义 Plugin 的生命周期 phases
- ✅ 处理好插件之间的依赖关系

### 5. 错误处理
- ✅ 使用 `AiError` 的语义化错误类型
- ✅ 保留原始错误信息
- ✅ 提供足够的上下文
- ✅ 区分可重试和不可重试的错误

---

## 性能考虑

### 零成本抽象
- Layer 使用静态分发 - 编译时解析
- 单次类型擦除 (`Arc<dyn Provider>`)
- 避免不必要的克隆

### 内存优化
- 使用 `Arc` 共享不可变数据
- 流式处理避免一次性加载大量数据
- 按需分配

### 异步性能
- 所有 I/O 操作都是异步的
- 使用 `tokio` 运行时
- 支持并发请求

---

## 总结

Aidale 采用分层架构设计，每一层都有明确的职责：

1. **Provider**: HTTP 客户端
2. **Strategy**: Provider 差异适配
3. **Runtime**: 请求编排
4. **Layer**: 横切关注点（AOP）
5. **Plugin**: 业务逻辑扩展

这种设计使得 Aidale 既强大又易于扩展，同时保持了优秀的性能和类型安全。
