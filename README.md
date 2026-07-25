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
- 实时频谱（spectrum）：FFT 1024 → 64 频段，以子像素连续曲线呈现，三段渐变色（青 / 品红 / 黄），中线为参考轴
- 即时查找（search）：键入字符实时过滤歌名 / 艺术家
- 进度条与当前曲目显示

## 构建

```sh
cargo build --release
# 二进制位于 target/release/tui-music
```

## 使用

```sh
./target/release/tui-music                 # 扫描 ~/Music
./target/release/tui-music -m /path/to/dir # 指定其他目录
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
| `q` / `Esc`  | 退出                |

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

播放器用 `MonoTee` 包装解码源：解码出的多声道样本下混为 mono，一边送往 cpal 播放，一边写入环形缓冲 `VizBuffer`。频谱模块从该缓冲取出最近的 1024 个采样，加汉宁窗后做 FFT，按频段聚合为 64 条柱状能量；UI 仅在每列顶端以 1/8 像素精度的 block 字符绘制顶部点，使其呈现为连续函数曲线。

## 测试

```sh
cargo test --release
```

包含音频扩展名识别与目录扫描两个集成测试。