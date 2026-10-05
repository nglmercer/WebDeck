use super::*;
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
