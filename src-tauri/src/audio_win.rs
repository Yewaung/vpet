//! Output volume level (0..1) of the default speakers, for jump-scare
//! reactions. This reads a single peak number from Windows' audio meter; it
//! never records, stores or analyses the audio itself, and never touches the
//! microphone.

#[cfg(windows)]
mod imp {
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

    /// Must stay on the thread that created it (COM).
    pub struct PeakMeter {
        meter: IAudioMeterInformation,
    }

    impl PeakMeter {
        pub fn open() -> Option<Self> {
            // SAFETY: standard COM activation of the default render endpoint's meter.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
                let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
                let meter: IAudioMeterInformation = device.Activate(CLSCTX_ALL, None).ok()?;
                Some(Self { meter })
            }
        }

        pub fn peak(&self) -> Option<f64> {
            // SAFETY: the meter interface is valid for this object's lifetime.
            unsafe { self.meter.GetPeakValue().ok().map(f64::from) }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct PeakMeter;
    impl PeakMeter {
        pub fn open() -> Option<Self> {
            None
        }
        pub fn peak(&self) -> Option<f64> {
            None
        }
    }
}

pub use imp::PeakMeter;

#[cfg(test)]
mod tests {
    /// `cargo test -p vpet peak_meter -- --ignored --nocapture` (needs an audio device)
    #[test]
    #[ignore]
    fn peak_meter_reads_a_level() {
        let m = super::PeakMeter::open().expect("default audio device");
        let v = m.peak().expect("peak value");
        println!("current output peak: {v:.3}");
        assert!((0.0..=1.0).contains(&v));
    }
}
