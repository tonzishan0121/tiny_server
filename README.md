# TinyServer

这是一个用rust编写的轻量化服务器程序。  
本项目承诺，不含一行人工代码，全程采用AI编程的方式编写。

## 本项目基础功能

1. 读取 `config/server.yaml` 配置监听地址、端口、worker 数量和连接参数。
2. 在代码中通过宏声明路由与 handler 的映射关系。
3. 支持基础 HTTP 请求解析和响应构造。
4. 支持固定线程池并发处理连接。
5. 支持 `/`、`/health`、`/index.html`、`/echo`。
6. 支持 `/chat`、`GET /api/messages`、`POST /api/messages`。
7. 支持 `/static/*` 静态目录访问，并提供基础 `Content-Type`。
8. 拒绝静态目录路径穿越。
9. 输出简单访问日志和错误日志。

## 运行方式

开发模式：

```bash
python3 run_dev.py
```

生产模式：

```bash
python3 run_prod.py
```

默认监听地址是 `http://127.0.0.1:7878`。

## 常用验证

```bash
curl http://127.0.0.1:7878/
curl http://127.0.0.1:7878/health
curl http://127.0.0.1:7878/index.html
curl http://127.0.0.1:7878/static/index.html
curl -X POST http://127.0.0.1:7878/echo -d 'hello'
curl http://127.0.0.1:7878/chat
curl http://127.0.0.1:7878/api/messages
```
