pub struct AudioInfo {
    pub samples_per_sec: u32,
}

impl AudioInfo {
    pub fn to_raw(&self) -> flick_obs_sys::obs_audio_info {
        // Both fields set explicitly (obs_audio_info has only these two).
        flick_obs_sys::obs_audio_info {
            samples_per_sec: self.samples_per_sec,
            speakers: flick_obs_sys::speaker_layout_SPEAKERS_STEREO,
        }
    }
}
