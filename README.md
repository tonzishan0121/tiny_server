# tiny_server

这算是一个满足遗憾的项目，我第一次学Cpp的时候最后目标其实就是把它做出来，这样一个可以运行在Linux平台的小型服务器程序，当然，现在我也不局限于Cpp了，改用rust来实现这样一个功能。

## 介绍

一个 **webserver（Web 服务器程序）** 的职责，本质上就是：**接收请求 → 处理请求 → 返回响应**。

### 一、接收和管理网络请求

Webserver 首先负责和客户端（浏览器、App等）建立通信。

* 监听端口（通常是 80 / 443）
* 接收 HTTP / HTTPS 请求
* 处理连接（TCP连接建立、关闭、复用）

### 二、解析 HTTP 请求

客户端发来的请求是原始数据，webserver需要理解它：

* 解析请求行（GET / POST / PUT 等）
* 解析请求头（Cookie、User-Agent、Authorization 等）
* 解析请求体（比如表单、JSON）


### 三、路由与分发请求

根据 URL 决定请求该交给谁处理：

* `/index.html` → 返回静态文件
* `/api/user` → 交给后端程序（如 Python / Node / Rust）

Webserver 通常直接处理：

* HTML
* CSS
* JavaScript
* 图片、视频

例如：

* Nginx
* Apache HTTP Server

### 五、与应用程序交互（动态内容）

对于动态请求：

* 转发给后端（如 Flask / Spring / Express）
* 或通过协议：

  * CGI / FastCGI
  * 反向代理（reverse proxy）

比如：

* 用户登录
* 数据库查询
* API 返回 JSON

### 六、构造并返回响应

处理完后，webserver负责返回标准 HTTP 响应：

* 状态码（200 / 404 / 500）
* 响应头
* 响应体（HTML / JSON 等）

### 七、安全与访问控制

Webserver 还承担基础安全职责：

* HTTPS（SSL/TLS 加密）
* 限制访问（IP、权限）
* 防止简单攻击（如请求洪泛）

### 八、性能优化

为了提高效率，它还会：

* 连接复用（Keep-Alive）
* 缓存（Cache-Control）
* 压缩（Gzip / Brotli）
* 负载均衡（分发请求到多个服务器）

### 九、日志与监控

记录系统运行情况：

* 访问日志（谁访问了什么）
* 错误日志（哪里出错了）

方便排查问题
