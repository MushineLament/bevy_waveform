use bevy::prelude::*;
use bevy_waveform::{
    ViewportRange, WaveformChannel, WaveformDecodeF32, WaveformImage, WaveformMaterial,
    WaveformParams, WaveformStyle,
};
use bevy_waveform::{
    WaveformPlugin, on_waveform_drag_end, on_waveform_drag_start, on_waveform_scroll,
};

#[derive(Component)]
pub struct AudioData(WaveformDecodeF32);

impl WaveformImage for AudioData {
    fn channel_samples(&self, view: &ViewportRange, width: u32) -> Vec<u8> {
        self.0.channel(0).channel_samples(view, width)
    }
}

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(WaveformPlugin::<AudioData>::new())
        .add_systems(Startup, setup)
        .run();
}
