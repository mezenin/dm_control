//! One bounded projection of Hex's existing microphone timeline. No second mic.
use super::{Settings, commands, detector::Detector};
use crate::dictation::indicator::{DictationIndicatorEvent, DictationIndicatorSender};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

struct Audio {
    samples: Vec<f32>,
    rate: u32,
    epoch: u64,
    at: Instant,
}

pub(crate) struct Controller {
    config: Settings,
    sender: mpsc::SyncSender<Audio>,
    stop: Arc<AtomicBool>,
    epoch: Arc<AtomicU64>,
    processing: Arc<AtomicBool>,
    listening: Arc<AtomicBool>,
    indicator: Option<DictationIndicatorSender>,
    busy: bool,
    worker: Option<JoinHandle<()>>,
}

impl Controller {
    pub(crate) fn reconcile(
        slot: &mut Option<Self>,
        indicator: Option<DictationIndicatorSender>,
        busy: bool,
    ) {
        let config = super::settings();
        if slot.as_ref().is_some_and(|c| c.config != config) || !config.enabled {
            *slot = None;
        }
        if !config.enabled {
            super::set_status("Off");
            return;
        }
        if slot.is_none() {
            *slot = Self::start(config, indicator);
        }
        if let Some(controller) = slot
            && busy != controller.busy
        {
            controller.busy = busy;
            controller.epoch.fetch_add(1, Ordering::AcqRel);
        }
    }

    fn start(config: Settings, indicator: Option<DictationIndicatorSender>) -> Option<Self> {
        let (sender, receiver) = mpsc::sync_channel(8);
        let stop = Arc::new(AtomicBool::new(false));
        let epoch = Arc::new(AtomicU64::new(0));
        let processing = Arc::new(AtomicBool::new(false));
        let listening = Arc::new(AtomicBool::new(false));
        let processing_worker = processing.clone();
        let listening_worker = listening.clone();
        let indicator_worker = indicator.clone();
        let stop_worker = stop.clone();
        let epoch_worker = epoch.clone();
        let phrase = config.wake_phrase.clone();
        super::set_status("Preparing wake phrase…");
        let worker = thread::Builder::new()
            .name("dm-voice-control".into())
            .spawn(move || {
                if let Err(error) = listen(
                    receiver,
                    &stop_worker,
                    &epoch_worker,
                    &phrase,
                    indicator_worker,
                    &processing_worker,
                    &listening_worker,
                ) && !stop_worker.load(Ordering::Acquire)
                {
                    super::set_status(format!("Voice control: {error:#}"));
                }
            })
            .map_err(|e| super::set_status(format!("Could not start voice control: {e}")))
            .ok()?;
        Some(Self {
            config,
            sender,
            stop,
            epoch,
            processing,
            listening,
            indicator,
            busy: false,
            worker: Some(worker),
        })
    }

    pub(crate) fn feed(&mut self, samples: &[f32], rate: u32, current: bool) {
        if !current {
            self.epoch.fetch_add(1, Ordering::AcqRel);
            return;
        }
        if self.busy || self.processing.load(Ordering::Acquire) {
            return;
        }
        let audio = Audio {
            samples: samples.to_vec(),
            rate,
            epoch: self.epoch.load(Ordering::Acquire),
            at: Instant::now(),
        };
        if self.sender.try_send(audio).is_err() {
            self.epoch.fetch_add(1, Ordering::AcqRel);
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.epoch.fetch_add(1, Ordering::AcqRel);
        if self.listening.swap(false, Ordering::AcqRel)
            && !self.busy
            && let Some(indicator) = &self.indicator
        {
            indicator.send(DictationIndicatorEvent::Discarded);
        }
        if let Some(worker) = self.worker.take().filter(|w| w.is_finished()) {
            let _ = worker.join();
        }
    }
}

struct Capture {
    samples: Vec<f32>,
    rate: u32,
    epoch: u64,
    speech_frames: usize,
    quiet_frames: usize,
}

impl Capture {
    fn push(&mut self, samples: &[f32]) -> bool {
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt();
        if rms > 0.008 {
            self.speech_frames += samples.len();
            self.quiet_frames = 0;
        } else {
            self.quiet_frames += samples.len();
        }
        self.samples.extend_from_slice(samples);
        let rate = self.rate as usize;
        (self.speech_frames >= rate / 5 && self.quiet_frames >= rate * 7 / 10)
            || self.samples.len() >= rate * 8
    }
}

fn listen(
    receiver: mpsc::Receiver<Audio>,
    stop: &AtomicBool,
    epoch: &AtomicU64,
    phrase: &str,
    indicator: Option<DictationIndicatorSender>,
    processing: &AtomicBool,
    listening: &AtomicBool,
) -> color_eyre::Result<()> {
    let mut detector = Detector::load(phrase)?;
    let mut capture = None::<Capture>;
    let mut preroll = VecDeque::<f32>::new();
    let mut generation = epoch.load(Ordering::Acquire);
    let mut sample_rate = 0;
    super::set_status(format!("Waiting for “{phrase}”"));
    while !stop.load(Ordering::Acquire) {
        let audio = match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(a) => a,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let now_epoch = epoch.load(Ordering::Acquire);
        if audio.epoch != now_epoch || audio.at.elapsed() > Duration::from_millis(300) {
            continue;
        }
        if generation != now_epoch || sample_rate != audio.rate {
            generation = now_epoch;
            sample_rate = audio.rate;
            detector.reset();
            capture = None;
            listening.store(false, Ordering::Release);
            preroll.clear();
        }
        if let Some(recording) = &mut capture {
            if let Some(i) = &indicator {
                i.meter(&audio.samples);
            }
            if recording.push(&audio.samples) {
                let recording = capture.take().expect("capture exists");
                listening.store(false, Ordering::Release);
                if let Some(i) = &indicator {
                    i.send(DictationIndicatorEvent::Discarded);
                }
                if recording.speech_frames >= recording.rate as usize / 5 {
                    processing.store(true, Ordering::Release);
                    process(recording, phrase, stop, epoch);
                    processing.store(false, Ordering::Release);
                } else {
                    super::set_status("No command heard");
                }
                detector.reset();
                preroll.clear();
            }
            continue;
        }
        preroll.extend(audio.samples.iter().copied());
        while preroll.len() > audio.rate as usize / 2 {
            preroll.pop_front();
        }
        if detector.feed(&audio.samples, audio.rate) {
            listening.store(true, Ordering::Release);
            super::set_status("Listening for a command…");
            if let Some(i) = &indicator {
                i.send(DictationIndicatorEvent::Started);
            }
            capture = Some(Capture {
                samples: preroll.drain(..).collect(),
                rate: audio.rate,
                epoch: now_epoch,
                speech_frames: 0,
                quiet_frames: 0,
            });
        }
    }
    Ok(())
}

fn process(capture: Capture, phrase: &str, stop: &AtomicBool, epoch: &AtomicU64) {
    let active = || {
        !stop.load(Ordering::Acquire)
            && super::enabled()
            && super::settings().wake_phrase == phrase
            && epoch.load(Ordering::Acquire) == capture.epoch
    };
    if !active() {
        return;
    }
    super::set_status("Recognizing command…");
    let result = (|| -> color_eyre::Result<String> {
        let samples = crate::dictation::try_resample_for_parakeet(&capture.samples, capture.rate)
            .map_err(|e| color_eyre::eyre::eyre!(e))?;
        let engine = crate::transcription::engine::TranscriptionEngine::shared()
            .ok_or_else(|| color_eyre::eyre::eyre!("The dictation model is not ready"))?;
        let transcript = engine
            .background(move |model| {
                model
                    .active()
                    .ok_or_else(|| color_eyre::eyre::eyre!("The speech model is unavailable"))?
                    .transcribe(samples)
            })?
            .ok_or_else(|| {
                color_eyre::eyre::eyre!("Dictation has priority; repeat the command")
            })??;
        if !active() {
            return Ok(String::new());
        }
        let Some(action) = commands::parse(&transcript, phrase) else {
            return Ok(format!(
                "Command not recognized: {}",
                transcript.chars().take(140).collect::<String>()
            ));
        };
        super::platform::execute(action)
    })();
    if active() {
        super::set_status(match result {
            Ok(message) => message,
            Err(e) => format!("Command failed: {e:#}"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inference_does_not_fill_the_audio_queue_or_cancel_itself() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut controller = Controller {
            config: Settings::default(),
            sender,
            stop: Arc::new(AtomicBool::new(false)),
            epoch: Arc::new(AtomicU64::new(0)),
            processing: Arc::new(AtomicBool::new(false)),
            listening: Arc::new(AtomicBool::new(false)),
            indicator: None,
            busy: false,
            worker: None,
        };
        controller.processing.store(true, Ordering::Release);
        for _ in 0..20 {
            controller.feed(&[0.0; 320], 16000, true);
        }
        assert!(receiver.try_recv().is_err());
        assert_eq!(controller.epoch.load(Ordering::Acquire), 0);
        controller.processing.store(false, Ordering::Release);
        controller.feed(&[0.0; 320], 16000, true);
        controller.feed(&[0.0; 320], 16000, true);
        assert_eq!(controller.epoch.load(Ordering::Acquire), 1);
        assert_eq!(receiver.try_recv().unwrap().epoch, 0);
        controller.feed(&[0.0; 320], 16000, false);
        assert_eq!(controller.epoch.load(Ordering::Acquire), 2);
    }
    #[test]
    fn command_capture_ends_after_speech_and_pause_or_timeout() {
        let mut capture = Capture {
            samples: vec![],
            rate: 16000,
            epoch: 0,
            speech_frames: 0,
            quiet_frames: 0,
        };
        assert!(!capture.push(&vec![0.05; 8000]));
        assert!(!capture.push(&vec![0.0; 8000]));
        assert!(capture.push(&vec![0.0; 4000]));
        let mut silent = Capture {
            samples: vec![],
            rate: 16000,
            epoch: 0,
            speech_frames: 0,
            quiet_frames: 0,
        };
        assert!(!silent.push(&vec![0.0; 16000]));
        assert!(silent.push(&vec![0.0; 7 * 16000]));
    }
}
