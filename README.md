# tui-music

一个用 Rust 编写的终端音乐播放器，扫描 `~/Music` 下的音频文件并以漂亮的 TUI 列表呈现，附带实时频谱可视化与即时查找。

## 功能

- 自动扫描音乐目录（mp3 / flac / wav / ogg / m4a / aac / opus / aiff / alac）
- 读取标签（标题 / 艺术家 / 时长），无标签时回退到文件名
- 播放控制（state）：暂停 / 播放
- 切换曲目（next / prev）：下一首、上一首
- 循环模式（repeat）：List 列表循环 / One 单曲循环 / Off 关闭
- 随机播放（shuffle）：On / Off
- 音量调节（volume）：±5%
- 歌单（playlist）：自动识别 `~/Music` 下目录名含「歌单」的子目录，目录内歌曲归入对应命名歌单；其余歌曲进入默认「歌曲架」按原逻辑平铺，`[` / `]` 切换歌单
- 实时频谱（spectrum）：FFT 2048 → 每列一个密集小点（无块），沿中线对称上下镜像，宽度填满（不丢最右），对数频率分布，柔和渐变色，激进 gamma 压缩 + attack/release 平滑
- 即时查找（search）：键入字符实时过滤歌名 / 艺术家
- 进度条与当前曲目显示

## 安装

### 通过 Homebrew（推荐，macOS / Linuxbrew）

本项目收录在作者个人 tap `orangerin/apps`，任何人都可直接安装：

```sh
brew tap orangerin/apps
brew install tui-music
```

或一行不 tap 直接装：

```sh
brew install orangerin/apps/tui-music
```

arm64 macOS 用户会自动下载预编译 bottle，无需本地编译；其他平台会从源码 `cargo build`。

升级：

```sh
brew upgrade tui-music
```

### 从源码构建

需要 Rust 工具链（`rustup` 或 Homebrew 的 `rust`）。

```sh
cargo build --release
# 二进制位于 target/release/tui-music
```

### 直接下载预编译二进制

到 [Releases](https://github.com/Shiorangerin/tui-music/releases) 下载对应平台的二进制，放入 `PATH` 即可。

## 使用

```sh
tui-music                                  # 扫描 ~/Music
tui-music -m /path/to/dir                  # 指定其他目录
./target/release/tui-music                 # 从源码构建后运行
```

## 快捷键

| 键           | 功能                |
| ------------ | ------------------- |
| `f` / `/`    | 进入查找模式        |
| 查找模式：键入 | 实时过滤歌名/艺术家 |
| 查找模式：`Enter` | 保留过滤退出     |
| 查找模式：`Esc`    | 清除过滤退出     |
| `j` / ↓      | 下移选择            |
| `k` / ↑      | 上移选择            |
| `Enter` / `l` | 播放选中曲目       |
| `Space`      | 暂停 / 继续         |
| `n` / Tab    | 下一首              |
| `p`          | 上一首              |
| `r`          | 切换循环模式        |
| `s`          | 开关随机播放        |
| `+` / `=`    | 音量 +5%            |
| `-`          | 音量 -5%            |
| `[` / `]`    | 上一个 / 下一个歌单  |
| `q` / `Esc`  | 退出                |

## 歌单

播放器启动时扫描 `~/Music`（或 `-m` 指定目录）：

- 凡是**直接子目录**名包含「歌单」的目录，其内所有音频文件（含嵌套子目录）自动归入一个命名歌单，歌单名去掉「歌单」后缀（如 `日语歌单` → `日语`）。
- 不属于任何歌单目录的音频文件全部进入默认的「歌曲架」歌单，按原有逻辑平铺排序。
- 空的歌单目录会被忽略，不会出现在标签栏。
- 在 TUI 顶部的歌单标签栏按 `[` / `]` 切换；搜索仅在当前活跃歌单内过滤。

## 技术栈

- **ratatui** + **crossterm** — TUI 渲染与终端事件
- **rodio** — 基于 cpal 的跨平台音频播放与解码
- **rustfft** — 实时频谱 FFT
- **lofty** — 标签与音频属性读取
- **walkdir** — 递归扫描音乐目录

## 架构

```
src/
├── lib.rs        库入口（可测试）
├── main.rs        TUI 事件循环 & 终端管理
├── app.rs         应用状态、键盘处理、查找过滤
├── ui.rs          ratatui 渲染（头部 / 查找栏 / 列表 / 频谱 / 状态栏）
├── library.rs    音乐目录扫描 & 标签读取  [库]
├── player.rs      rodio 播放封装 + VizBuffer 分流 [库]
└── viz.rs         FFT 频谱计算           [库]
```

播放器用 `MonoTee` 包装解码源：解码出的多声道样本下混为 mono，一边送往 cpal 播放，一边写入环形缓冲 `VizBuffer`。频谱模块从该缓冲取出最近的 2048 个采样，加汉宁窗后做 FFT，按对数频率聚合为终端宽度自适应条数；UI 在中线两侧以 1/8 像素精度 block 字符绘制镜像细线。app 内对每个频段做 attack(0.45)/release(0.08) 帧间平滑，让起伏自然不抖。

## 测试

```sh
cargo test --release
```

包含音频扩展名识别与目录扫描两个集成测试。