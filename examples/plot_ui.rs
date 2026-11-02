use bevy::camera::NormalizedRenderTarget;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::prelude::*;
use bevy_waveform::WaveformPlugin;
use bevy_waveform::load::WaveformDecodeF32;
use bevy_waveform::render::{
    ViewportRange, WaveformMaterial, WaveformParams, WaveformRangeDrag, WaveformStyle,
};

#[derive(Component)]
pub struct AudioData(WaveformDecodeF32);

pub const AUDIO: &str = r".\assets\audio.wav";

fn setup(
    mut commands: Commands,
    mut ui_materials: ResMut<Assets<WaveformMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn(Camera2d);

    let audio = WaveformDecodeF32::from_path(AUDIO, None).unwrap();
    let total = audio.channel(0).len();

    let range = ViewportRange::full(total);
    let style = WaveformStyle::default();

    let image = audio.waveform_texture(0, &range, style.texture_width);
    let tex = images.add(image);

    let mat = ui_materials.add(WaveformMaterial {
        waveform: tex.clone(),
        params: WaveformParams::build_params(&style),
    });

    commands
        .spawn((
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
        // .observe(on_waveform_drag)
        .observe(on_waveform_drag_start)
        .observe(on_waveform_drag_end)
        .observe(on_waveform_scroll);
}

fn update_texture_on_range_change(
    query: Query<
        (
            &AudioData,
            &ViewportRange,
            &WaveformStyle,
            &MaterialNode<WaveformMaterial>,
        ),
        Changed<ViewportRange>,
    >,
    mut images: ResMut<Assets<Image>>,
    mut ui_materials: ResMut<Assets<WaveformMaterial>>,
) {
    for (audio, range, style, mat_node) in query.iter() {
        if let Some(material) = ui_materials.get_mut(&mat_node.0)
            && let Some(mut image) = images.get_mut(&material.waveform)
        {
            image.data = Some(WaveformDecodeF32::generate_waveform_data(
                audio.0.channel(0),
                range,
                style.texture_width,
            ));
        }
    }
}

fn update_uniform_on_view_or_style_change(
    query: Query<(&WaveformStyle, &MaterialNode<WaveformMaterial>), Changed<WaveformStyle>>,
    mut ui_materials: ResMut<Assets<WaveformMaterial>>,
) {
    for (style, mat_node) in query.iter() {
        if let Some(mut material) = ui_materials.get_mut(&mat_node.0) {
            material.params = WaveformParams::build_params(style);
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(WaveformPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_texture_on_range_change,
                update_uniform_on_view_or_style_change,
            )
                .chain(),
        )
        .add_systems(PostUpdate, on_waveform_drag_system)
        // .add_systems(PostUpdate, handle_input)
        .run();
}

/// 拖拽开始：给实体打上 WaveformRangeDrag 标记
fn on_waveform_drag_start(drag_start: On<PointerDragStart>, mut commands: Commands) {
    commands.entity(drag_start.entity).insert(WaveformRangeDrag);
}

/// 拖拽结束：移除标记
fn on_waveform_drag_end(drag_end: On<PointerDragEnd>, mut commands: Commands) {
    commands
        .entity(drag_end.entity)
        .remove::<WaveformRangeDrag>();
}

fn on_waveform_scroll(
    scroll: On<PointerScroll>,
    mut query: Query<(&AudioData, &mut ViewportRange)>,
) {
    let Ok((audio, mut range)) = query.get_mut(scroll.entity) else {
        return;
    };
    let total = audio.0.data[0].len();
    if total == 0 {
        return;
    }

    // 根据滚动单位归一化，使不同设备的缩放体验一致
    let scroll_amount = match scroll.unit {
        MouseScrollUnit::Line => scroll.y * 0.15, // 行模式，一格约15%
        MouseScrollUnit::Pixel => scroll.y * 0.002, // 像素模式，更精细
    };
    // 使用 exp 保证缩放的正负对称性，避免来回缩放产生漂移
    let zoom_factor = scroll_amount.exp();

    // 以当前视口中心为焦点进行缩放
    let center = range.min_index + range.span() / 2;
    range.zoom_at(zoom_factor, center, total);
}

const SPEED_FACTOR: f32 = 10000.0;

fn on_waveform_drag_system(
    pointer_state: Res<PointerState>,
    pointer_locations: Query<(&PointerId, &PointerLocation)>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut query: Query<(&AudioData, &mut ViewportRange, &ComputedNode), With<WaveformRangeDrag>>,
    windows: Query<&Window>,
) {
    // 1. 检查鼠标左键是否在拖拽，并找到被拖拽的实体
    let dragged_entity = pointer_state
        .pointer_buttons
        .get(&(PointerId::Mouse, PointerButton::Primary))
        .and_then(|state| state.dragging.keys().next().copied());

    let Some(dragged_entity) = dragged_entity else {
        // 未拖拽时也要消费掉消息，避免积压
        for _ in mouse_motion.read() {}
        return;
    };

    // 2. 拿到对应实体的音频、视口、布局
    let Ok((audio, mut range, node)) = query.get_mut(dragged_entity) else {
        for _ in mouse_motion.read() {}
        return;
    };

    let total = audio.0.channel(0).len();
    if total == 0 {
        for _ in mouse_motion.read() {}
        return;
    }

    // 3. 获取指针当前所在窗口和位置
    let Some((_, pointer_loc)) = pointer_locations
        .iter()
        .find(|(id, _)| **id == PointerId::Mouse)
    else {
        for _ in mouse_motion.read() {}
        return;
    };
    let Some(location) = pointer_loc.location() else {
        for _ in mouse_motion.read() {}
        return;
    };
    let NormalizedRenderTarget::Window(window_ref) = &location.target else {
        for _ in mouse_motion.read() {}
        return;
    };
    let Ok(window) = windows.get(window_ref.entity()) else {
        for _ in mouse_motion.read() {}
        return;
    };

    // 4. 累计本帧所有 MouseMotion 的 delta
    let dx: f32 = mouse_motion.read().map(|e| e.delta.x).sum();

    // 5. 计算 cursor_x
    let logical_pixels = window.physical_width() as f32 / window.scale_factor();
    let ratio = location.position.x / logical_pixels - 0.5;

    let cursor_x = match ratio {
        n if n < -0.3 => (n + 0.3) / 0.2,
        n if n > 0.3 => (n - 0.3) / 0.2,
        _ => {
            // 中心死区：只做常规拖拽
            let width_px = node.size().x.max(1.0);
            let samples_per_px = range.span() as f32 / width_px;
            range.pan((-dx * samples_per_px) as isize, total);
            return;
        }
    }
    .clamp(-1.0, 1.0);

    // 6. 边缘自动滚动
    let delta = (cursor_x * SPEED_FACTOR) as isize;
    if delta != 0 {
        range.pan(delta, total);
    }
}
