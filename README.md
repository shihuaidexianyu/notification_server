# notification_server

一个 Rust HTTP 服务：接收统一通知请求（`service/title/to/body`），并按服务类型发送通知。

当前已实现服务类型：`smtp`（兼容别名 `stmp`）。

## 1. 配置

复制配置模板并填写：

```bash
cp .env.example .env
```

加载环境变量（示例）：

```bash
set -a
source .env
set +a
```

## 2. 启动

```bash
cargo run
```

默认监听：`127.0.0.1:8080`。

## 3. 接口

### 健康检查

```bash
curl http://127.0.0.1:8080/healthz
```

### 发送通知

- 路径：`POST /notify`（`/send-notification` 同样可用）
- 鉴权：
  - `x-api-key: <API_KEY>`
  - 或 `Authorization: Bearer <API_KEY>`

```bash
curl -X POST http://127.0.0.1:8080/notify \
  -H 'Content-Type: application/json' \
  -H 'x-api-key: change_me' \
  -d '{
    "service": "smtp",
    "title": "测试标题",
    "to": "receiver@example.com",
    "body": "这是一封测试邮件"
  }'
```

成功返回：

```json
{"ok":true,"message":"sent"}
```

常见失败：

- `401`：API key 错误或缺失
- `400`：参数不合法（空标题/空正文/无效收件人）
- `500`：SMTP 发送失败
