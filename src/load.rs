use std::{num::NonZeroU32, path::Path};

use bevy::{
    image::Image,
    prelude::{Deref, DerefMut},
};
use symphonium::{DecodedAudio, DecodedAudioF32, error::LoadError};

use crate::{ChannelF32Ref, ViewportRange, WaveformImage};

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

    pub fn get_channel(&self, index: usize) -> Option<ChannelF32Ref<'_>> {
        self.data.get(index).map(Vec::as_slice).map(ChannelF32Ref)
    }

    pub fn channel(&self, index: usize) -> ChannelF32Ref<'_> {
        ChannelF32Ref(&self.data[index])
    }

    pub fn data(&self) -> &Vec<Vec<f32>> {
        &self.data
    }

    pub fn waveform_texture(&self, index: usize, view: &ViewportRange, width: u32) -> Image {
        self.channel(index).create_image(view, width)
    }
}
