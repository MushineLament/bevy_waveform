use bevy::{
    asset::{Asset, Handle, uuid_handle},
    image::Image,
    math::Vec4,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::{Shader, ShaderRef},
    sprite_render::{AlphaMode2d, Material2d},
    ui_render::ui_material::UiMaterial,
};

use crate::WaveformStyle;

pub const WAVEFORM_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("00000000-0000-0000-1211-c985222427a2");

pub const WAVEFORM_UI_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("00000000-0000-0000-1211-c985222427a3");

#[derive(ShaderType, Clone)]
pub struct WaveformParams {
    /// x: texture_width, y: line_width(px), z: 预留, w: padding_px
    pub info: Vec4,
    pub color: Vec4,
    /// x: scale_y (振幅缩放 0.05~4.0)
    pub transform: Vec4,
}

impl WaveformParams {
    /// 只依赖 style：所有 uniform 参数（宽度、线宽、颜色、边距、振幅缩放）都在样式层
    pub fn build_params(style: &WaveformStyle) -> Self {
        Self {
            info: Vec4::new(
                style.texture_width as f32,
                style.line_width,
                0.0,
                style.padding_px,
            ),
            color: style.color,
            transform: Vec4::new(style.scale_y.clamp(0.05, 4.0), 0.0, 0.0, 0.0),
        }
    }
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct WaveformMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub waveform: Handle<Image>,
    #[uniform(2)]
    pub params: WaveformParams,
}

impl Material2d for WaveformMaterial {
    fn fragment_shader() -> ShaderRef {
        WAVEFORM_SHADER_HANDLE.into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

impl UiMaterial for WaveformMaterial {
    fn fragment_shader() -> ShaderRef {
        WAVEFORM_UI_SHADER_HANDLE.into()
    }
}
