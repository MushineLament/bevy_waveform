# bevy_waveform

Bevy 0.20 的音频波形渲染库，支持 **Mesh2d** 和 **UI Node** 两种渲染载体，内置滚轮锚点缩放与拖拽平移（含边缘自动滚动）。

---

## ✨ 特性

- 🎨 **一套材质，两种载体**：`WaveformMaterial` 同时实现 `Material2d` 和 `UiMaterial`，波形可以画在 `Mesh2d` 上，也可以画在 UI `Node` 上
- 🎯 **锚点缩放**：滚轮以光标位置对应的采样点为焦点缩放，视觉锚定稳定
- 🖐 **拖拽平移**：鼠标左键拖拽移动视口，支持鼠标静止时的**边缘自动滚动**
- 🚀 **速度统一**：UI 与 Mesh 两种载体的拖拽手感一致，以「节点屏幕逻辑像素宽度」为分母
- 📦 **开箱即用**：着色器通过 `uuid_handle!` + `load_internal_asset!` 编译进二进制，无运行时文件依赖
- 🎵 **多格式解码**：基于 [`symphonium`](https://crates.io/crates/symphonium)，支持 WAV / OGG，可通过 feature 扩展 MP3 / FLAC
- 🧩 **泛型音频源**：实现 `WaveformSource` trait 即可接入自定义采样数据，插件按类型自动注册

---

## 📦 依赖

```toml
[dependencies]
bevy = "0.20"
bevy_waveform = { git = "https://github.com/MushineLament/bevy_waveform.git" }
```

> 要求 **Rust 1.85+**（`edition = "2024"`）。

---

## 🚀 快速开始

### 1. 定义音频源组件

实现 `WaveformSource` trait，返回你希望可视化的**单声道采样切片**：

```rust
use bevy::prelude::*;
use bevy_waveform::{WaveformDecodeF32, WaveformSource};

#[derive(Component)]
pub struct AudioData(WaveformDecodeF32);

impl WaveformSource for AudioData {
    fn channel_samples(&self) -> &[f32] {
        self.0.channel(0)  // 左声道
    }
}
```

### 2. 注册插件

```rust
use bevy::prelude::*;
use bevy_waveform::WaveformPlugin;

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(MeshPickingPlugin)                       // 需要 Picking 交互
    .add_plugins(WaveformPlugin::<AudioData>::new())      // 泛型 = 你的音频组件
    .add_systems(Startup, setup)
    .run();
```

### 3. 生成波形实体

#### 方式 A：`Mesh2d`（世界空间）

```rust
use bevy_waveform::{
    ViewportRange, WaveformParams, WaveformStyle,
    on_waveform_drag_start, on_waveform_drag_end, on_waveform_scroll,
};

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WaveformMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn(Camera2d);

    let audio = WaveformDecodeF32::from_path("assets/audio.wav", None).unwrap();
    let total = audio.channel(0).len();

    let range = ViewportRange::full(total);
    let style = WaveformStyle::default();

    let image = audio.waveform_texture(0, &range, style.texture_width);
    let tex = images.add(image);

    let mat = materials.add(WaveformMaterial {
        waveform: tex.clone(),
        params: WaveformParams::build_params(&style),
    });

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(800.0, 200.0))),
        MeshMaterial2d(mat),
        Transform::default(),
        Visibility::default(),
        range,
        style,
        AudioData(audio),
    ))
    .observe(on_waveform_drag_start)
    .observe(on_waveform_drag_end)
    .observe(on_waveform_scroll);
}
```

#### 方式 B：UI `Node`（屏幕空间）

```rust
commands.spawn((
    Node {
        width: Val::Percent(50.0),
        height: Val::Percent(50.0),
        ..default()
    },
    MaterialNode(mat),
    range,
    style,
    AudioData(audio),
))
.observe(on_waveform_drag_start)
.observe(on_waveform_drag_end)
.observe(on_waveform_scroll);
```

---

## 🧠 核心概念

### `ViewportRange` —— 时间轴视口

```rust
pub struct ViewportRange {
    pub size: usize,   // 音频总采样数
    pub min: usize,    // 视口起点（采样索引）
    pub max: usize,    // 视口终点（采样索引）
}
```

| 方法 | 作用 |
|:---|:---|
| `full(size)` | 构造全量视口 |
| `span()` | 当前跨度 = `max - min` |
| `pan(delta)` | 平移 `delta` 个采样，内部 clamp 到 `[0, size]` |
| `zoom_at(factor, focus_index)` | 以 `focus_index` 为焦点缩放，`factor > 1` 放大 |
| `set_full()` | 重置为全量 |

### `WaveformStyle` —— 视觉参数

```rust
pub struct WaveformStyle {
    pub line_width: f32,       // 线条宽度（像素）
    pub color: Vec4,           // 波形颜色
    pub padding_px: f32,       // 上下留白
    pub scale_y: f32,          // 振幅缩放，1.0 = 原始幅度
    pub texture_width: u32,    // 纹理分辨率（像素列数）
}
```

修改后会自动触发 uniform 更新与纹理重建，无需手动刷新。

### `WaveformSource` —— 自定义音频源

```rust
pub trait WaveformSource: Component {
    fn channel_samples(&self) -> &[f32];
}
```

任何组件实现此 trait 后，`WaveformPlugin::<T>` 会自动为其注册纹理更新系统。多声道文件可以通过 `audio.channel(n)` 选择任意声道。

---

## 🎮 交互

| 输入 | 效果 |
|:---|:---|
| **鼠标滚轮** | 以光标位置为锚点缩放视口 |
| **鼠标左键拖拽** | 平移视口（内容跟随鼠标） |
| **拖到节点边缘 20% 区域** | 自动滚动，越靠近边缘速度越快 |
| `←` / `→` | 平移视口（可选，见示例） |
| `+` / `-` | 缩放视口 |
| `↑` / `↓` | 调整振幅缩放 |
| `R` | 重置为全量视口 |
| `[` / `]` | 调整纹理分辨率（减半 / 翻倍） |

**拖拽与静止**：拖拽状态由 `WaveformRangeDrag` 标记维护，`on_waveform_drag_system` 每帧轮询，因此**鼠标静止在边缘时仍会持续滚动**。

---

## 🏗️ 架构

```
src/
├── lib.rs                 # 公共导出
├── components.rs          # ViewportRange / WaveformStyle / WaveformRangeDrag
├── load.rs                # WaveformDecodeF32：symphonium 解码封装
├── plugin.rs              # WaveformPlugin / WaveformBasePlugin / 交互系统
├── render/
│   ├── mod.rs
│   ├── waveform.rs        # WaveformMaterial / WaveformParams / 着色器 Handle
│   └── shaders/
│       ├── waveform.wesl      # Material2d 版
│       └── waveform_ui.wesl   # UiMaterial 版
```

**着色器内嵌**：`WAVEFORM_SHADER_HANDLE` 与 `WAVEFORM_UI_SHADER_HANDLE` 由 `uuid_handle!` 生成，`load_internal_asset!` 在 `WaveformBasePlugin` 中编译进二进制。调用方**不需要**在 `assets/shaders/` 中放任何 `.wesl` 文件。

**抗锯齿与线条连续性**：着色器用**线段距离场**（而非纯垂直距离）计算覆盖率，放大到单采样级别时线条依然连续，无断点。

---

## 📝 注意事项

1. **`Mesh2d` 宽度需与 `MESH_WIDTH` 一致**：`on_waveform_drag_system` 里 `MESH_WIDTH = 800.0` 用于换算采样/像素比例。如果你的网格宽度不是 800，修改 `plugin.rs` 中的常量，或改为通过 `Transform.scale.x` 动态推导（当前已支持 scale）。

2. **多相机场景**：Mesh 分支的拖拽速度换算使用 `camera_query.iter().next()`。多相机场景建议添加 Marker 组件筛选出渲染波形的那一个。

3. **`WaveformPlugin<T>` 与 `WaveformBasePlugin`**：
   - `WaveformPlugin::<T>::new()` 会自动注册 `WaveformBasePlugin`（着色器 + 材质管线），再追加 `update_texture_on_range_change::<T>`。
   - 如果你要注册多个音频类型（`AudioData` 和另一个 `VideoAudio`），可以分别 `add_plugins(WaveformPlugin::<AudioData>::new())` 和 `add_plugins(WaveformPlugin::<VideoAudio>::new())`，内部会自动去重。

4. **没有音频时拖拽的边界**：`on_waveform_drag_system` 在 `dragged_entity` 为 `None` 时提前返回，但**没有消费 `MouseMotion` 消息**。如果发现后续拖拽时有一次性的位置跳变，可以在所有早退分支前加 `for _ in mouse_motion.read() {}`。

---

## 📄 许可证

MIT OR Apache-2.0

---

## 🔗 相关资源

- [Bevy 官方文档](https://docs.rs/bevy/0.20)
- [Symphonium](https://crates.io/crates/symphonium) —— 本库的解码后端
- [Bevy Picking](https://docs.rs/bevy_picking) —— 交互系统底层
- [WESL 着色语言](https://github.com/bevyengine/naga_oil) —— 着色器编写

---

**项目状态**：早期开发阶段（`0.1.0`）。API 可能随 Bevy 版本升级调整，欢迎 issue / PR。

## 📄 补充
**1**: 以上由AI生成文档(临时性使用).
**2**：经过思考，在当前时间节点下，AI能够翻译中英文绝大多数一般内容，即优先使用母语保证表达准确性
