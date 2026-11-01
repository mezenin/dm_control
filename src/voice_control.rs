//! DM Control's opt-in, local wake-phrase and multimedia command mode.
//! Ordinary dictation always has priority. No transcript becomes shell code.

mod commands;
#[cfg(target_os = "macos")]
mod detector;
#[cfg(target_os = "macos")]
mod platform;
#[cfg(target_os = "macos")]
mod runtime;

use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, RwLock};

#[cfg(target_os = "macos")]
pub(crate) use runtime::Controller;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub wake_phrase: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            wake_phrase: "Hello Hex".into(),
        }
    }
}

static SETTINGS: LazyLock<RwLock<Settings>> = LazyLock::new(|| RwLock::new(Settings::default()));
static STATUS: LazyLock<RwLock<String>> = LazyLock::new(|| RwLock::new("Off".into()));

pub fn configure(settings: Settings) {
    *SETTINGS.write().unwrap_or_else(|e| e.into_inner()) = settings;
}

pub fn settings() -> Settings {
    SETTINGS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn enabled() -> bool {
    cfg!(target_os = "macos") && settings().enabled
}

pub fn status() -> String {
    STATUS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

fn set_status(message: impl Into<String>) {
    *STATUS.write().unwrap_or_else(|e| e.into_inner()) = message.into();
}

pub fn validate_phrase(phrase: &str) -> Result<(), &'static str> {
    let words: Vec<_> = phrase.split_whitespace().collect();
    if !(2..=5).contains(&words.len()) || phrase.len() > 64 {
        return Err("Use an English wake phrase of 2–5 words (up to 64 characters)");
    }
    if !phrase
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c.is_ascii_whitespace() || c == '\'')
    {
        return Err(
            "The wake-phrase model supports English letters; commands can be Russian or English",
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn check(
    text: Option<&str>,
    wav: Option<&std::path::Path>,
    phrase: &str,
) -> color_eyre::Result<()> {
    use color_eyre::eyre::eyre;
    validate_phrase(phrase).map_err(|e| eyre!(e))?;
    if let Some(text) = text {
        let action = commands::parse(text, phrase);
        println!(
            "{}",
            serde_json::json!({"recognized":action.is_some(), "action":action.map(|a| format!("{a:?}")), "executed":false})
        );
        return Ok(());
    }
    if let Some(wav) = wav {
        let reader = hound::WavReader::open(wav)?;
        let spec = reader.spec();
        if spec.channels != 1
            || !(8_000..=192_000).contains(&spec.sample_rate)
            || spec.bits_per_sample != 16
            || u64::from(reader.duration()) > u64::from(spec.sample_rate) * 120
        {
            return Err(eyre!(
                "Use a mono 16-bit PCM WAV no longer than 120 seconds"
            ));
        }
        let mut samples = reader
            .into_samples::<i16>()
            .map(|n| n.map(|n| f32::from(n) / 32768.0))
            .collect::<Result<Vec<_>, _>>()?;
        samples.extend(std::iter::repeat_n(0.0, spec.sample_rate as usize));
        let mut detector = detector::Detector::load(phrase)?;
        let mut detections = 0;
        for chunk in samples.chunks((spec.sample_rate as usize / 50).max(1)) {
            if detector.feed(chunk, spec.sample_rate) {
                detections += 1;
            }
        }
        println!(
            "{}",
            serde_json::json!({"wake_phrase":phrase, "detections":detections, "executed":false})
        );
        return Ok(());
    }
    println!(
        "Use --text 'сделай тише' to inspect a command, or --wav speech.wav to check a wake phrase. No action is executed."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_opt_in_and_phrase_validation_is_bounded() {
        assert!(!Settings::default().enabled);
        assert!(validate_phrase("Hello Hex").is_ok());
        assert!(validate_phrase("Hey Computer").is_ok());
        for invalid in [
            "",
            "Hex",
            "Привет Хекс",
            "Hello Hex\n; reboot",
            "a b c d e f",
        ] {
            assert!(validate_phrase(invalid).is_err(), "{invalid}");
        }
    }
}
