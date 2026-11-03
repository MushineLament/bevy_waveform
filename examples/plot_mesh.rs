use bevy::prelude::*;
use bevy_waveform::{
    ViewportRange, WaveformDecodeF32, WaveformMaterial, WaveformParams, WaveformStyle,
};
use bevy_waveform::{
    WaveformPlugin, WaveformSource, on_waveform_drag_end, on_waveform_drag_start,
    on_waveform_scroll,
};

#[derive(Component)]
pub struct AudioData(WaveformDecodeF32);

impl WaveformSource for AudioData {
    fn channel_samples(&self) -> &[f32] {
        self.0.channel(0)
    }
}

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

    commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::new(800.0, 200.0))),
            MeshMaterial2d(mat),
            Transform::default().with_scale(Vec3::new(1.0, 1.0, 1.0)),
            Visibility::default(),
            range,
            style,
            AudioData(audio),
        ))
        .observe(on_waveform_drag_start)
        .observe(on_waveform_drag_end)
        .observe(on_waveform_scroll);
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(WaveformPlugin::<AudioData>::new())
        .add_systems(Startup, setup)
        .add_systems(PostUpdate, (handle_input,))
        .run();
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut ViewportRange, &mut WaveformStyle)>,
) {
    let Ok((mut range, mut style)) = query.single_mut() else {
        return;
    };

    // ← → 平移（改 ViewportRange）
    if keys.pressed(KeyCode::ArrowRight) {
        let span = range.span();
        range.pan(span as isize / 20);
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        let span = range.span();
        range.pan(-(span as isize / 20));
    }

    // + / - X 轴缩放（改 ViewportRange）
    let center = range.min + range.span() / 2;
    if keys.just_pressed(KeyCode::Equal) {
        range.zoom_at(1.5, center);
    }
    if keys.just_pressed(KeyCode::Minus) {
        range.zoom_at(1.0 / 1.5, center);
    }

    // R 全量（改 ViewportRange）
    if keys.just_pressed(KeyCode::KeyR) {
        range.set_full();
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
