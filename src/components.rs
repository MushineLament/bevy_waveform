use bevy::{ecs::component::Component, math::Vec4};

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct WaveformRangeDrag;

/// 视口：X 轴时间范围 + 纹理宽度
/// 注意：scale_y 已移除，振幅缩放属于视觉样式，见 WaveformStyle
#[derive(Component, Clone, Copy, Debug)]
pub struct ViewportRange {
    pub size: usize,
    pub min: usize,
    pub max: usize,
}

impl ViewportRange {
    pub fn full(size: usize) -> Self {
        Self {
            size,
            min: 0,
            max: size.max(1),
        }
    }

    /// 视口跨度（= max - min），最小为 1
    pub fn span(&self) -> usize {
        self.max.saturating_sub(self.min).max(1)
    }

    /// 相对当前视口平移 `delta` 个采样。自动 clamp 到 [0, size] 范围。
    pub fn pan(&mut self, delta: isize) {
        let span = self.span();
        let new_min = (self.min as isize + delta)
            .clamp(0, (self.size.saturating_sub(span)) as isize) as usize;
        self.min = new_min;
        self.max = (new_min + span).min(self.size);
    }

    /// 以 `focus_index`（采样索引）为中心缩放。
    /// `factor > 1.0` 放大（跨度缩小），`factor < 1.0` 缩小（跨度增大）。
    /// 缩放后焦点采样在屏幕上的相对位置保持不变。
    pub fn zoom_at(&mut self, factor: f32, focus_index: usize) {
        let old_span = self.span();
        let new_span = ((old_span as f32 / factor) as usize).clamp(64, self.size);
        if new_span == old_span {
            return;
        }

        // 焦点在旧视口内的相对位置
        let rel = (focus_index.saturating_sub(self.min)) as f32 / old_span as f32;
        // 新视口的起点：让 focus 在新视口内仍保持相对位置 rel
        let new_min_f = focus_index as f32 - rel * new_span as f32;
        let new_min = new_min_f.clamp(0.0, (self.size.saturating_sub(new_span)) as f32) as usize;

        self.min = new_min;
        self.max = (new_min + new_span).min(self.size);
    }

    /// 重置为全量视口
    pub fn set_full(&mut self) {
        self.min = 0;
        self.max = self.size.max(1);
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
