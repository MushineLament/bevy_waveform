use std::{num::NonZeroU32, path::Path};

use bevy::{
    asset::RenderAssetUsages,
    image::{Image, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::{Deref, DerefMut},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use symphonium::{DecodedAudio, DecodedAudioF32, error::LoadError};

use crate::render::ViewportRange;

#[derive(Deref, DerefMut)]
pub struct WaveformDecode(pub DecodedAudio);

impl WaveformDecode {
    pub fn from_path<T: AsRef<Path>>(
        path: T,
        target_sample_rate: Option<NonZeroU32>,
    ) -> Result<Self, LoadError> {
        let probed = symphonium::probe_from_file(path, None)?;

        Ok(Self(symphonium::decode(
            probed,
            &symphonium::DecodeConfig::default(),
            target_sample_rate,
            None,
            None,
        )?))
    }
}

#[derive(Clone, Deref, DerefMut)]
pub struct WaveformDecodeF32(pub DecodedAudioF32);

impl WaveformDecodeF32 {
    pub fn from_path<T: AsRef<Path>>(
        path: T,
        target_sample_rate: Option<NonZeroU32>,
    ) -> Result<Self, LoadError> {
        let probed = symphonium::probe_from_file(path, None)?;

        Ok(Self(symphonium::decode_f32(
            probed,
            &symphonium::DecodeConfig::default(),
            target_sample_rate,
            None,
            None,
        )?))
    }

    pub fn get_channel(&self, index: usize) -> Option<&[f32]> {
        self.data.get(index).map(Vec::as_slice)
    }

    pub fn channel(&self, index: usize) -> &[f32] {
        &self.data[index]
    }

    pub fn data(&self) -> &Vec<Vec<f32>> {
        &self.data
    }

    pub fn generate_waveform_data(channel: &[f32], view: &ViewportRange, width: u32) -> Vec<u8> {
        let total = channel.len();
        let lo = view.min_index.min(total);
        let hi = view.max_index.min(total).max(lo);
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

    pub fn waveform_texture(&self, index: usize, view: &ViewportRange, width: u32) -> Image {
        Self::crate_waveform_texture(self.channel(index), view, width)
    }

    pub fn crate_waveform_texture(channel: &[f32], view: &ViewportRange, width: u32) -> Image {
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
        image.data = Some(Self::generate_waveform_data(channel, view, width));
        image
    }
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
        let mut mins = Vec::with_capacity(target_width);
        let mut maxs = Vec::with_capacity(target_width);
        let max_idx = (source.len() - 1).max(1) as f32;
        for i in 0..target_width {
            let t = if target_width == 1 {
                0.0
            } else {
                i as f32 / (target_width - 1) as f32
            };
            let pos = t * max_idx;
            let i0 = pos.floor() as usize;
            let i1 = (i0 + 1).min(source.len() - 1);
            let frac = pos - i0 as f32;
            let v = source[i0] * (1.0 - frac) + source[i1] * frac;
            mins.push(v);
            maxs.push(v);
        }
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
            let mut mn = source[start];
            let mut mx = source[start];
            for &s in &source[start..end] {
                if s < mn {
                    mn = s;
                }
                if s > mx {
                    mx = s;
                }
            }
            mins.push(mn);
            maxs.push(mx);
        }
        (mins, maxs)
    }
}
