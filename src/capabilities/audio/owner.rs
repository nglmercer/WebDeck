use super::*;
use rodio::Source;
type Sound = (rodio::MixerDeviceSink, rodio::Player);
#[derive(Default)]
pub(in crate::capabilities) struct AudioOwner {
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    closed: bool,
    sounds: Vec<Sound>,
}
impl AudioOwner {
    pub(in crate::capabilities) fn play(
        &self,
        targets: Vec<rodio::cpal::Device>,
        path: &std::path::Path,
        volume: u64,
        start_ms: u64,
        end_ms: Option<u64>,
        context: &Context,
    ) -> Result<Value> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        context.check(Capability::Audio)?;
        if state.closed {
            return Err(Error::new(ErrorCode::ShuttingDown, "Audio owner is closed"));
        }
        state.sounds.retain(|(_, player)| !player.empty());
        if state.sounds.len() + targets.len() > 32 {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Sound capacity exhausted",
            ));
        }
        // Prepare every output while paused. A later failure cannot leave earlier outputs playing.
        let mut prepared = Vec::new();
        for device in targets {
            context.check(Capability::Audio)?;
            let sink = rodio::DeviceSinkBuilder::from_device(device)
                .map_err(|_| Error::execution())?
                .open_stream()
                .map_err(|_| Error::execution())?;
            let player = rodio::Player::connect_new(sink.mixer());
            player.pause();
            let source =
                rodio::Decoder::try_from(fs::File::open(path).map_err(|_| Error::execution())?)
                    .map_err(|_| Error::execution())?;
            let source = fragment(source, start_ms, end_ms)?;
            player.set_volume(volume as f32 / 100.0);
            player.append(source);
            prepared.push((sink, player));
        }
        context.check(Capability::Audio)?;
        for (_, player) in &prepared {
            player.play();
        }
        state.sounds.extend(prepared);
        Ok(json!({"playing":true}))
    }
    pub(in crate::capabilities) fn stop(&self) {
        self.clear(false);
    }
    pub(in crate::capabilities) fn shutdown(&self) {
        self.clear(true);
    }
    fn clear(&self, close: bool) {
        let sounds = {
            let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
            state.closed |= close;
            std::mem::take(&mut state.sounds)
        };
        for (_, player) in &sounds {
            player.stop();
        }
        drop(sounds);
    }
}
impl Drop for AudioOwner {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn fragment<S: Source + Send + 'static>(
    source: S,
    start_ms: u64,
    end_ms: Option<u64>,
) -> Result<Box<dyn Source + Send>> {
    if end_ms.is_some_and(|end| end <= start_ms) {
        return Err(Error::invalid());
    }
    if source.total_duration().is_some_and(|duration| {
        Duration::from_millis(start_ms) >= duration
            || end_ms.is_some_and(|end| Duration::from_millis(end) > duration)
    }) {
        return Err(Error::new(
            ErrorCode::InvalidInput,
            "Audio fragment exceeds the file duration",
        ));
    }
    let source = source.skip_duration(Duration::from_millis(start_ms));
    Ok(if let Some(end) = end_ms {
        Box::new(source.take_duration(Duration::from_millis(end - start_ms)))
    } else {
        Box::new(source)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone() -> rodio::buffer::SamplesBuffer {
        let source = rodio::source::SineWave::new(440.0);
        let channels = source.channels();
        let sample_rate = source.sample_rate();
        let samples = source
            .take_duration(Duration::from_secs(1))
            .collect::<Vec<_>>();
        rodio::buffer::SamplesBuffer::new(channels, sample_rate, samples)
    }
    #[test]
    fn fragment_plays_only_the_selected_samples_without_an_audio_device() {
        let all: Vec<_> = tone().collect();
        let cut: Vec<_> = fragment(tone(), 250, Some(750)).unwrap().collect();
        assert_eq!(cut.len(), all.len() / 2);
        assert_eq!(cut, all[all.len() / 4..all.len() * 3 / 4]);
        assert_eq!(fragment(tone(), 0, None).unwrap().collect::<Vec<_>>(), all);
    }
    #[test]
    fn fragment_rejects_reversed_empty_and_out_of_file_ranges() {
        for (start, end) in [
            (500, Some(500)),
            (750, Some(250)),
            (1000, None),
            (0, Some(1001)),
        ] {
            assert!(fragment(tone(), start, end).is_err());
        }
    }
}
