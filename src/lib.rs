pub mod render;

pub mod load;

use bevy::app::{App, Plugin};
use bevy::asset::{Assets, Handle, load_internal_asset, uuid_handle};
use bevy::log::info;
use bevy::shader::Shader;
use bevy::sprite_render::Material2dPlugin;
use bevy::ui_render::UiMaterialPlugin;

use crate::render::WaveformMaterial;

pub const WAVEFORM_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("00000000-0000-0000-1211-c985222427a2");

pub const WAVEFORM_UI_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("00000000-0000-0000-1211-c985222427a3");

pub struct WaveformPlugin;

impl Plugin for WaveformPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WAVEFORM_SHADER_HANDLE,
            "shaders/waveform.wesl",
            Shader::from_wesl
        );
        load_internal_asset!(
            app,
            WAVEFORM_UI_SHADER_HANDLE,
            "shaders/waveform_ui.wesl",
            Shader::from_wesl
        );

        if !app.is_plugin_added::<Material2dPlugin<WaveformMaterial>>() {
            app.add_plugins(Material2dPlugin::<WaveformMaterial>::default());
        }

        if !app.is_plugin_added::<UiMaterialPlugin<WaveformMaterial>>() {
            app.add_plugins(UiMaterialPlugin::<WaveformMaterial>::default());
        }

        // 在 WaveformPlugin::build 的最后
        let assets = app.world().resource::<Assets<Shader>>();
        let state = assets.get(&WAVEFORM_SHADER_HANDLE);
        info!("Shader in Assets: {:?}", state.is_some());
    }
}
