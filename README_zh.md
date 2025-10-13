# Daedalus CLI

[![](https://img.shields.io/crates/v/daedalus-cli)](https://crates.io/crates/daedalus-cli)
[![](https://docs.rs/daedalus-cli/badge.svg)](https://docs.rs/daedalus-cli)
[![](https://img.shields.io/crates/l/daedalus-cli)](https://github.com/Zephyruston/daedalus-cli/blob/main/LICENSE)

[English Documentation](README.md) | [英文文档](README.md)

在数据迷宫中塑造您的路径。

Daedalus CLI 是一个基于 Rust 的 PostgreSQL 数据库管理和探索的命令行界面工具。它提供了一个直观的终端用户界面（TUI），允许用户连接到 PostgreSQL 数据库，浏览表格，并使用分页支持查看数据。

## 特性

- **数据库连接管理**：使用加密存储在 `~/.daedalus-cli/config.json` 中的连接信息，可以添加、列出和删除已保存的数据库连接
- **终端用户界面**：直观的 TUI 用于浏览数据库表，使用箭头键在当前视图中的项目之间导航
- **数据探索**：浏览数据库表，显示列标题，并以行突出显示的方式查看表数据
- **分页支持**：使用 PageUp/PageDown 键导航大型数据集
- **自定义 SQL 查询**：在 TUI 中直接执行自定义 SQL 查询并分页显示结果
- **安全存储**：使用 AES-256-GCM 加密存储敏感连接密码信息

## 安装

从 crates.io 安装 Daedalus CLI：

```bash
cargo install daedalus-cli
```

或从源代码构建：

```bash
git clone https://github.com/Zephyruston/daedalus-cli.git
cd daedalus-cli
cargo install --path .
```

## 使用方法

### 添加数据库连接

使用自定义名称添加新的数据库连接：

```bash
daedalus-cli add-conn postgresql://username:password@host:port/database --name mydb
```

或让系统根据主机和数据库生成名称：

```bash
daedalus-cli add-conn postgresql://username:password@host:port/database
```

### 列出已保存的连接

列出所有已保存的数据库连接：

```bash
daedalus-cli list-conns
```

### 删除连接

删除已保存的连接：

```bash
daedalus-cli remove-conn mydb
```

### 连接到数据库

使用 TUI 连接到已保存的数据库：

```bash
daedalus-cli connect mydb
```

### 测试连接

测试连接而不打开 TUI：

```bash
daedalus-cli ping mydb
```

### 生成 Shell 补全

为 bash、zsh 和 fish 生成命令行补全脚本：

```bash
# 为 bash 生成补全脚本
daedalus-cli completions bash

# 为 zsh 生成补全脚本
daedalus-cli completions zsh

# 为 fish 生成补全脚本
daedalus-cli completions fish
```

要使补全在当前会话中生效，您需要根据您的 shell 类型执行以下操作之一：

**对于 bash：**

```bash
# 生成并保存补全脚本
daedalus-cli completions bash > ~/.bash_completion.d/daedalus-cli
# 在 .bashrc 中引用文件
echo "source ~/.bash_completion.d/daedalus-cli" >> ~/.bashrc
# 重新加载配置
source ~/.bashrc
```

**对于 zsh：**

```bash
# 将补全脚本生成到标准位置
daedalus-cli completions zsh > /usr/local/share/zsh/site-functions/_daedalus-cli
# 或用于用户特定的安装
mkdir -p ~/.zsh/completion
daedalus-cli completions zsh > ~/.zsh/completion/_daedalus-cli
echo "fpath+=~/.zsh/completion" >> ~/.zshrc
# 重新加载配置
source ~/.zshrc
```

**对于 fish：**

```bash
# 将补全脚本生成到 fish 的补全目录
mkdir -p ~/.config/fish/completions
daedalus-cli completions fish > ~/.config/fish/completions/daedalus-cli.fish
# 重新启动 fish 或重新加载配置
```

## TUI 导航

连接到数据库后，TUI 提供以下导航控制：

- **箭头键 (↑/↓)**：在当前视图中的项目之间导航
- **Enter**：选择高亮的项目
- **PageUp/PageDown**：在大型数据集中导航
- **'s'**：进入自定义 SQL 查询模式或返回查询输入
- **'t'**：返回表列表
- **'c'**：返回连接选择
- **'q' 或 Esc**：退出应用程序

## 自定义 SQL 查询

Daedalus CLI 现在支持直接从 TUI 执行自定义 SQL 查询：

- **进入查询模式**：从表列表视图中按 's' 键进入自定义查询输入模式
- **执行查询**：输入 SQL 查询并按 Enter 键执行
- **查看结果**：查询结果以分页表格格式显示
- **导航结果**：使用箭头键在行之间导航，使用 PageUp/PageDown 键切换页面
- **查询输入**：查询输入区域支持文本编辑和光标移动（左/右，Home/End）
- **返回查询输入**：查看结果时按 's' 键返回查询输入界面

## 安全性

Daedalus CLI 实施安全措施以保护您的数据库凭据：

- 连接密码在存储到配置文件之前使用 AES-256-GCM 加密
- 随机生成的加密密钥存储在 `~/.daedalus-cli/key.bin` 中
- 所有连接均使用安全的 tokio-postgres 库建立

## 开发

### 先决条件

- Rust edition 2024
- PostgreSQL 服务器连接（或使用提供的 Docker 容器）

### 测试

```bash
# 运行测试
cargo test

# 检查编译错误
cargo check

# 运行 linter
cargo clippy
```

### Docker 支持

该项目包含一个 `docker-compose.yml` 文件，用于设置带有示例数据的 PostgreSQL 容器以进行测试：

```bash
# 使用示例数据启动 PostgreSQL 容器
docker-compose up -d

# 停止容器
docker-compose down
```

#### 使用 Docker 测试数据库

1. **启动测试数据库**：

   ```bash
   docker-compose up -d
   ```

2. **等待数据库初始化**：

   - 容器启动后，数据库需要一些时间来初始化和运行
   - 您可以使用 `docker-compose logs -f` 查看启动进度
   - 默认情况下，数据库将在端口 5432 上运行

3. **连接到测试数据库**：

   ```bash
   daedalus-cli add-conn postgresql://test:123456@localhost:5432/test_db --name test_db
   ```

4. **测试连接**：

   ```bash
   daedalus-cli ping test_db
   ```

5. **浏览数据**：
   ```bash
   daedalus-cli connect test_db
   ```

#### 测试数据库结构

Docker 测试数据库包含以下示例表以供演示：

- **users**：存储用户信息（id, username, email, created_at）
- **projects**：存储项目信息（id, name, description, owner_id）
- **tasks**：存储任务信息（id, title, description, project_id, assigned_to, status, priority）
- **api_keys**：存储 API 密钥（id, key_value, user_id, name, permissions）

这些表包含样本数据，允许您在真实数据库环境中测试 Daedalus CLI 功能。

#### 停止测试数据库

完成后，您可以停止数据库容器：

```bash
docker-compose down
```

这将停止所有运行的容器，但保留数据卷。如果您需要删除数据卷（清除所有数据），请使用：

```bash
docker-compose down -v
```

## 配置

连接信息存储在 `~/.daedalus-cli/config.json` 中。这包括：

- 主机、端口、数据库名、用户名
- 加密的密码数据
- 用于标识的连接名

加密密钥存储在 `~/.daedalus-cli/key.bin` 中，应妥善保管。

## 许可证

MIT 许可证 - 详情请参见 [LICENSE](LICENSE) 文件。
