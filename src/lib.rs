pub mod render;

pub mod load;

use bevy::app::{App, Plugin};
use bevy::asset::embedded_asset;
use bevy::sprite_render::Material2dPlugin;

use crate::render::WaveformMaterial;

pub struct WaveformPlugin;

impl Plugin for WaveformPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "../assets/shaders/waveform.wesl");
        embedded_asset!(app, "../assets/shaders/waveform_ui.wesl");

        // 注册两套材质管线
        if !app.is_plugin_added::<Material2dPlugin<WaveformMaterial>>() {
            app.add_plugins(Material2dPlugin::<WaveformMaterial>::default());
        }
    }
}
