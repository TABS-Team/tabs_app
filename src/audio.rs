use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::prelude::*;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::FromFileError;
use kira::{AudioManager, AudioManagerSettings, DefaultBackend, PlaySoundError};
use thiserror::Error;

#[derive(Resource)]
pub struct StreamingAudio {
    manager: AudioManager<DefaultBackend>,
}

impl FromWorld for StreamingAudio {
    fn from_world(_world: &mut World) -> Self {
        let manager =
            AudioManager::new(AudioManagerSettings::default()).expect("Failed to init audio");
        Self { manager }
    }
}

#[derive(Debug, Error)]
pub enum StreamingAudioError {
    #[error("failed to load streaming audio from {path}: {source}")]
    Load {
        path: PathBuf,
        #[source]
        source: FromFileError,
    },
    #[error("failed to start streaming playback: {0}")]
    Play(#[from] PlaySoundError<FromFileError>),
}

impl StreamingAudio {
    const FALLBACK_DURATION_SECS: f64 = 3600.0;

    fn duration_from_codec_params(
        params: &symphonia::core::codecs::CodecParameters,
    ) -> Option<f64> {
        let sample_rate = params.sample_rate?;
        let n_frames = params.n_frames?;
        Some(n_frames as f64 / sample_rate as f64)
    }

    fn estimate_duration_secs(path: &Path) -> Option<f64> {
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;
        use symphonia::default::get_probe;

        let file = std::fs::File::open(path).ok()?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(ext);
        }

        let probed = get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .ok()?;
        let format = probed.format;
        let track = format.default_track()?;
        Self::duration_from_codec_params(&track.codec_params)
    }

    fn prepare_stream_data(
        path: &Path,
    ) -> Result<StreamingSoundData<FromFileError>, StreamingAudioError> {
        let data =
            StreamingSoundData::from_file(path).map_err(|source| StreamingAudioError::Load {
                path: path.to_path_buf(),
                source,
            })?;

        let decoder_duration = data.duration();
        if decoder_duration > Duration::from_secs(0) {
            let secs = decoder_duration.as_secs_f64();
            info!(
                "StreamingSoundData duration (decoder reported): {:.3}s",
                secs
            );
            if secs.is_finite() && secs >= 1.0 {
                return Ok(data);
            }
        }

        if let Some(duration_secs) = Self::estimate_duration_secs(path) {
            info!(
                "Estimated duration via metadata for {}: {:.3}s",
                path.display(),
                duration_secs
            );
            if duration_secs.is_finite() && duration_secs > 0.0 {
                return Ok(data.slice(0.0..duration_secs));
            }
        } else {
            warn!("Could not estimate duration for {}", path.display());
        }

        warn!(
            "Falling back to default streaming duration of {:.0}s for {}",
            Self::FALLBACK_DURATION_SECS,
            path.display()
        );
        Ok(data.slice(0.0..Self::FALLBACK_DURATION_SECS))
    }

    pub fn drain_backend_errors(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        while let Some(err) = self.manager.backend_mut().pop_error() {
            error!("Audio backend error: {err}");
        }
    }

    pub fn play_from_path(
        &mut self,
        path: &Path,
    ) -> Result<StreamingSoundHandle<FromFileError>, StreamingAudioError> {
        let data = Self::prepare_stream_data(path)?;
        Ok(self.manager.play(data)?)
    }
}
