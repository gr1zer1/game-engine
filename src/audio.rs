use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Cursor},
    path::Path,
    sync::Arc,
    time::Duration,
};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

enum SoundClip {
    // Keep bytes in memory so playback has no file IO.
    FileBytes(Arc<[u8]>),
    // Synthesized fallback tone for simple UI sounds.
    Tone {
        frequency_hz: u32,
        duration: Duration,
    },
}

pub struct AudioEngine {
    // Must stay alive for the whole engine lifetime, or audio output stops.
    _stream: OutputStream,
    handle: OutputStreamHandle,
    clips: HashMap<String, SoundClip>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|err| format!("audio device init failed: {err}"))?;

        Ok(Self {
            _stream: stream,
            handle,
            clips: HashMap::new(),
        })
    }

    pub fn register_sound_file(
        &mut self,
        sound_id: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        let path = path.as_ref();
        let bytes = fs::read(path)
            .map_err(|err| format!("failed to read sound '{}': {err}", path.display()))?;
        self.clips
            .insert(sound_id.into(), SoundClip::FileBytes(bytes.into()));
        Ok(())
    }

    pub fn register_tone(
        &mut self,
        sound_id: impl Into<String>,
        frequency_hz: u32,
        duration_ms: u64,
    ) {
        self.clips.insert(
            sound_id.into(),
            SoundClip::Tone {
                frequency_hz,
                duration: Duration::from_millis(duration_ms.max(1)),
            },
        );
    }

    pub fn play(&self, sound_id: &str, volume: f32) -> Result<(), String> {
        let clip = self
            .clips
            .get(sound_id)
            .ok_or_else(|| format!("unknown sound id '{sound_id}'"))?;
        let volume = volume.max(0.0);

        match clip {
            SoundClip::FileBytes(bytes) => {
                let cursor = Cursor::new(bytes.clone());
                let decoder = Decoder::new(BufReader::new(cursor))
                    .map_err(|err| format!("failed to decode sound '{sound_id}': {err}"))?;

                let sink = Sink::try_new(&self.handle)
                    .map_err(|err| format!("failed to create audio sink: {err}"))?;
                sink.set_volume(volume);
                sink.append(decoder);
                // Detach so playback continues after this function returns.
                sink.detach();
            }
            SoundClip::Tone {
                frequency_hz,
                duration,
            } => {
                let sink = Sink::try_new(&self.handle)
                    .map_err(|err| format!("failed to create audio sink: {err}"))?;
                sink.set_volume(volume);
                sink.append(
                    rodio::source::SineWave::new(*frequency_hz as f32)
                        .take_duration(*duration)
                        .amplify(0.20),
                );
                // Detach so playback continues after this function returns.
                sink.detach();
            }
        }

        Ok(())
    }
}
