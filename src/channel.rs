use bevy::{
    asset::RenderAssetUsages,
    image::{Image, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::{Deref, DerefMut},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::ViewportRange;

#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut)]
pub struct ChannelF32(pub Vec<f32>);

impl WaveformChannel for ChannelF32 {
    fn channel(&self) -> &[f32] {
        self.as_slice()
    }
}

impl ChannelF32 {
    pub fn as_ref(&self) -> ChannelF32Ref<'_> {
        ChannelF32Ref(self.0.as_slice())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut)]
pub struct ChannelF32Ref<'a>(pub &'a [f32]);

impl<'a> WaveformChannel for ChannelF32Ref<'a> {
    fn channel(&self) -> &[f32] {
        self.0
    }
}

pub trait WaveformChannel {
    fn channel(&self) -> &[f32];
}

pub trait WaveformImage {
    fn create_image(&self, view: &ViewportRange, width: u32) -> Image;
    fn channel_samples(&self, view: &ViewportRange, width: u32) -> Vec<u8>;
}

impl WaveformImage for ChannelF32Ref<'_> {
    fn create_image(&self, view: &ViewportRange, width: u32) -> Image {
        let mut image = Image::new_fill(
            Extent3d {
                width,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[128u8, 128, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Nearest,
            min_filter: ImageFilterMode::Nearest,
            ..Default::default()
        });
        image.data = Some(self.channel_samples(view, width));
        image
    }

    fn channel_samples(&self, view: &ViewportRange, width: u32) -> Vec<u8> {
        generate_waveform_data(self.channel(), view, width)
    }
}

impl WaveformImage for ChannelF32 {
    fn create_image(&self, view: &ViewportRange, width: u32) -> Image {
        self.as_ref().create_image(view, width)
    }

    fn channel_samples(&self, view: &ViewportRange, width: u32) -> Vec<u8> {
        self.as_ref().channel_samples(view, width)
    }
}

pub fn generate_waveform_data(channel: &[f32], view: &ViewportRange, width: u32) -> Vec<u8> {
    let total = channel.len();
    let lo = view.min.min(total);
    let hi = view.max.min(total).max(lo);
    let slice = &channel[lo..hi];

    let (mins, maxs) = resample_min_max_to_width(slice, width as usize);

    let mut bytes = Vec::with_capacity(mins.len() * 4);
    for i in 0..mins.len() {
        // R = min, G = max
        let mn_u = ((mins[i].clamp(-1.0, 1.0) + 1.0) * 127.5) as u8;
        let mx_u = ((maxs[i].clamp(-1.0, 1.0) + 1.0) * 127.5) as u8;
        bytes.extend_from_slice(&[mn_u, mx_u, 0, 255]);
    }
    bytes
}

/// 把任意长度的采样降到 `target_width` 个点，返回 (min_series, max_series)。
/// - 放大（source ≤ target）：线性插值，min == max（退化为点连线）
/// - 缩小（source > target）：每段同时取 min 和 max（包络）
pub fn resample_min_max_to_width(source: &[f32], target_width: usize) -> (Vec<f32>, Vec<f32>) {
    if target_width == 0 {
        return (Vec::new(), Vec::new());
    }
    if source.is_empty() {
        return (vec![0.0; target_width], vec![0.0; target_width]);
    }

    if source.len() <= target_width {
        // 放大：线性插值，min = max（点连线）
        let max_idx = (source.len() - 1).max(1) as f32;

        let inv = if target_width == 1 {
            0.0
        } else {
            1.0 / (target_width - 1) as f32
        } * max_idx;

        let vec = (0..target_width)
            .map(|i| {
                let pos = i as f32 * inv;
                let i0 = pos.floor() as usize;
                let i1 = (i0 + 1).min(source.len() - 1);
                let frac = pos - i0 as f32;
                source[i0] * (1.0 - frac) + source[i1] * frac
            })
            .collect::<Vec<f32>>();

        let mins = vec.clone();
        let maxs = vec;

        (mins, maxs)
    } else {
        // 缩小：每段取 min 和 max（真实包络，不稀疏）
        let step = source.len() as f64 / target_width as f64;
        let mut mins = Vec::with_capacity(target_width);
        let mut maxs = Vec::with_capacity(target_width);
        for i in 0..target_width {
            let start = (i as f64 * step) as usize;
            let end = (((i + 1) as f64 * step) as usize).min(source.len());
            if start >= end {
                mins.push(0.0);
                maxs.push(0.0);
                continue;
            }
            let slice = &source[start..end];
            let mn = slice.iter().copied().fold(f32::INFINITY, f32::min);
            let mx = slice.iter().copied().fold(f32::NEG_INFINITY, f32::max);

            mins.push(mn);
            maxs.push(mx);
        }
        (mins, maxs)
    }
}
