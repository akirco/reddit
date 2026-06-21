# Reddit CLI

一个使用 Rust 开发的命令行工具，让你直接在终端浏览 Reddit。通过 Reddit 公开 JSON API 获取数据,并且支持下载。

## 构建

```bash
# 开发模式
cargo build

# 发布模式（优化构建）
cargo build --release
```

## 运行

```bash
cargo run -- [OPTIONS]
```

## 环境变量

- `REDDIT_COOKIE`:Reddit 认证 Cookie，用于访问受限内容
