use bevy::{
    asset::{Asset, Handle},
    ecs::component::Component,
    image::Image,
    math::Vec4,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
    ui_render::ui_material::UiMaterial,
};

use crate::{WAVEFORM_SHADER_HANDLE, WAVEFORM_UI_SHADER_HANDLE};

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct WaveformRangeDrag;

/// 视口：X 轴时间范围 + 纹理宽度
/// 注意：scale_y 已移除，振幅缩放属于视觉样式，见 WaveformStyle
#[derive(Component, Clone, Copy, Debug)]
pub struct ViewportRange {
    pub min_index: usize,
    pub max_index: usize,
}

impl ViewportRange {
    pub fn full(total: usize) -> Self {
        Self {
            min_index: 0,
            max_index: total.max(1),
        }
    }

    pub fn span(&self) -> usize {
        self.max_index.saturating_sub(self.min_index).max(1)
    }

    pub fn pan(&mut self, delta: isize, total: usize) {
        let span = self.span();
        let new_min = (self.min_index as isize + delta)
            .clamp(0, (total.saturating_sub(span)) as isize) as usize;
        self.min_index = new_min;
        self.max_index = (new_min + span).min(total);
    }

    pub fn zoom_at(&mut self, factor: f32, focus_index: usize, total: usize) {
        let old_span = self.span();
        let new_span = ((old_span as f32 / factor) as usize).clamp(64, total);
        if new_span == old_span {
            return;
        }
        let rel = (focus_index.saturating_sub(self.min_index)) as f32 / old_span as f32;
        let new_min_f = focus_index as f32 - rel * new_span as f32;
        let new_min = new_min_f.clamp(0.0, (total.saturating_sub(new_span)) as f32) as usize;
        self.min_index = new_min;
        self.max_index = (new_min + new_span).min(total);
    }

    pub fn set_full(&mut self, total: usize) {
        self.min_index = 0;
        self.max_index = total.max(1);
    }
}

/// 视觉样式：线宽、颜色、边距、振幅缩放
/// 升级为 Component，便于 handle_input 修改 scale_y
#[derive(Component, Clone)]
pub struct WaveformStyle {
    pub line_width: f32,
    pub color: Vec4,
    pub padding_px: f32,
    /// 振幅缩放：1.0 = 原始幅度，>1.0 放大，<1.0 缩小
    pub scale_y: f32,
    pub texture_width: u32,
}

impl Default for WaveformStyle {
    fn default() -> Self {
        Self {
            line_width: 1.5,
            color: Vec4::new(0.2, 1.0, 0.4, 1.0),
            padding_px: 8.0,
            scale_y: 1.0,
            texture_width: 1024,
        }
    }
}

impl WaveformStyle {
    pub fn adjust_scale_y(&mut self, factor: f32) {
        self.scale_y = (self.scale_y * factor).clamp(0.05, 4.0);
    }

    pub fn set_scale_y(&mut self, v: f32) {
        self.scale_y = v.clamp(0.05, 4.0);
    }

    pub fn set_texture_width(&mut self, width: u32) {
        self.texture_width = width;
    }
}

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
