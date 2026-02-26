# stmp_server

一个 Rust HTTP 服务：接收 `title/to/body`，并通过已配置的 SMTP 邮箱发送邮件。

## 1. 配置

复制配置模板并填写：

```bash
cp .env.example .env
```

然后设置环境变量（示例）：

```bash
set -a
source .env
set +a
```

## 2. 启动

```bash
cargo run
```

默认监听：`127.0.0.1:8080`

## 3. 接口

### 健康检查

```bash
curl http://127.0.0.1:8080/healthz
```

### 发送邮件

```bash
curl -X POST http://127.0.0.1:8080/send-email \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "测试标题",
    "to": "receiver@example.com",
    "body": "这是一封测试邮件"
  }'
```

成功返回：

```json
{"ok":true,"message":"sent"}
```
