use bevy::prelude::*;
use bevy_waveform::WaveformPlugin;
use bevy_waveform::load::WaveformDecodeF32;
use bevy_waveform::render::{ViewportRange, WaveformMaterial, WaveformParams, WaveformStyle};

#[derive(Resource)]
pub struct AudioData(WaveformDecodeF32);

pub const AUDIO: &str = r".\assets\audio.wav";

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WaveformMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn(Camera2d);

    let audio = WaveformDecodeF32::from_path(AUDIO, None).unwrap();

    let total = audio.channel(0).len();
    info!("加载音频: {} 采样 @ {} Hz", total, audio.sample_rate);

    // ✅ ViewportRange 只剩时间范围
    let range = ViewportRange::full(total);
    // ✅ texture_width 在 style 里
    let style = WaveformStyle::default();

    // ✅ 宽度从 style 取
    let image = audio.waveform_texture(0, &range, style.texture_width);
    let tex = images.add(image);

    // ✅ build_params 只依赖 style
    let params = WaveformParams::build_params(&style);
    let mat = materials.add(WaveformMaterial {
        waveform: tex.clone(),
        params,
    });

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(800.0, 200.0))),
        MeshMaterial2d(mat),
        Transform::default().with_scale(Vec3::new(1.0, 1.0, 1.0)),
        Visibility::default(),
        range,
        style,
    ));

    commands.insert_resource(AudioData(audio));
}

fn update_texture_on_range_change(
    audio: Res<AudioData>,
    // ✅ Query 加上 &WaveformStyle，需要读 texture_width
    query: Query<(
        &ViewportRange,
        &WaveformStyle,
        &MeshMaterial2d<WaveformMaterial>,
    )>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<WaveformMaterial>>,
    // ✅ 缓存 key 加上 texture_width：宽度变化也要重建
    mut last: Local<Option<(usize, usize, u32)>>,
) {
    let Ok((range, style, mat_handle)) = query.single() else {
        return;
    };

    let current = (range.min_index, range.max_index, style.texture_width);
    if *last == Some(current) {
        return;
    }
    *last = Some(current);

    if let Some(material) = materials.get_mut(&mat_handle.0)
        && let Some(mut image) = images.get_mut(&material.waveform)
    {
        // ✅ 只生成 data，不新建 Image
        image.data = Some(WaveformDecodeF32::generate_waveform_data(
            audio.0.channel(0),
            range,
            style.texture_width,
        ));
    }
}

/// ✅ 样式变化 → 更新 uniform
fn update_uniform_on_view_or_style_change(
    query: Query<(&WaveformStyle, &MeshMaterial2d<WaveformMaterial>)>,
    mut materials: ResMut<Assets<WaveformMaterial>>,
    // ✅ 顺序改为 (texture_width, scale_y, line_width, color, padding_px)
    mut last: Local<Option<(u32, f32, f32, [f32; 4], f32)>>,
) {
    let Ok((style, mat_handle)) = query.single() else {
        return;
    };

    let key = (
        style.texture_width,    // u32
        style.scale_y,          // f32
        style.line_width,       // f32
        style.color.to_array(), // [f32; 4]
        style.padding_px,       // f32
    );
    if *last == Some(key) {
        return;
    }
    *last = Some(key);

    if let Some(mut material) = materials.get_mut(&mat_handle.0) {
        material.params = WaveformParams::build_params(style);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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
        .add_systems(PostUpdate, handle_input)
        .run();
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut ViewportRange, &mut WaveformStyle)>,
    audio: Res<AudioData>,
) {
    let Ok((mut range, mut style)) = query.single_mut() else {
        return;
    };
    let total = audio.0.data[0].len();

    // ← → 平移（改 ViewportRange）
    if keys.pressed(KeyCode::ArrowRight) {
        let span = range.span();
        range.pan(span as isize / 20, total);
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        let span = range.span();
        range.pan(-(span as isize / 20), total);
    }

    // + / - X 轴缩放（改 ViewportRange）
    let center = range.min_index + range.span() / 2;
    if keys.just_pressed(KeyCode::Equal) {
        range.zoom_at(1.5, center, total);
    }
    if keys.just_pressed(KeyCode::Minus) {
        range.zoom_at(1.0 / 1.5, center, total);
    }

    // R 全量（改 ViewportRange）
    if keys.just_pressed(KeyCode::KeyR) {
        range.set_full(total);
    }

    // ↑ / ↓ Y 轴振幅缩放（改 WaveformStyle）
    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::PageUp) {
        style.adjust_scale_y(1.2);
        info!("scale_y = {:.2}", style.scale_y);
    }
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::PageDown) {
        style.adjust_scale_y(1.0 / 1.2);
        info!("scale_y = {:.2}", style.scale_y);
    }

    // 可选：纹理分辨率调节示例（[ / ] 减半 / 翻倍）
    if keys.just_pressed(KeyCode::BracketRight) {
        let value = style.texture_width * 2;
        style.set_texture_width((value).min(4096));
        info!("texture_width = {}", style.texture_width);
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        let value = style.texture_width / 2;
        style.set_texture_width((value).max(256));
        info!("texture_width = {}", style.texture_width);
    }
}
