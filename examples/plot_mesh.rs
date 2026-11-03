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
        .run();
}
