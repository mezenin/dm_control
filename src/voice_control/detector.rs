use color_eyre::eyre::{Result, eyre};
use sentencepiece_rs::SentencePieceProcessor;
use sherpa_onnx::{KeywordSpotter, KeywordSpotterConfig, OnlineStream};
use std::path::PathBuf;

pub(super) struct Detector {
    model: KeywordSpotter,
    stream: OnlineStream,
}

fn materialize() -> Result<PathBuf> {
    let folder = crate::app_paths::support_dir()?.join("wake-model-v1");
    std::fs::create_dir_all(&folder)?;
    let files: &[(&str, &[u8])] = &[
        (
            "encoder.onnx",
            include_bytes!("../../resources/voice-control/encoder.onnx"),
        ),
        (
            "decoder.onnx",
            include_bytes!("../../resources/voice-control/decoder.onnx"),
        ),
        (
            "joiner.onnx",
            include_bytes!("../../resources/voice-control/joiner.onnx"),
        ),
        (
            "tokens.txt",
            include_bytes!("../../resources/voice-control/tokens.txt"),
        ),
        (
            "bpe.model",
            include_bytes!("../../resources/voice-control/bpe.model"),
        ),
    ];
    for (name, bytes) in files {
        let path = folder.join(name);
        if std::fs::read(&path).ok().as_deref() != Some(*bytes) {
            std::fs::write(path, bytes)?;
        }
    }
    Ok(folder)
}

impl Detector {
    pub(super) fn load(phrase: &str) -> Result<Self> {
        super::validate_phrase(phrase).map_err(|e| eyre!(e))?;
        let folder = materialize()?;
        let tokenizer = SentencePieceProcessor::open(folder.join("bpe.model"))?;
        let pieces = tokenizer.encode(&phrase.to_ascii_uppercase())?;
        if pieces.iter().any(|p| p == "<unk>") {
            return Err(eyre!(
                "The wake phrase contains a word this model cannot tokenize"
            ));
        }
        let mut config = KeywordSpotterConfig::default();
        config.model_config.transducer.encoder =
            Some(folder.join("encoder.onnx").to_string_lossy().into());
        config.model_config.transducer.decoder =
            Some(folder.join("decoder.onnx").to_string_lossy().into());
        config.model_config.transducer.joiner =
            Some(folder.join("joiner.onnx").to_string_lossy().into());
        config.model_config.tokens = Some(folder.join("tokens.txt").to_string_lossy().into());
        config.model_config.num_threads = 1;
        config.model_config.provider = Some("cpu".into());
        config.keywords_threshold = 0.5;
        config.keywords_buf = Some(format!("{} @wake\n", pieces.join(" ")));
        let model = KeywordSpotter::create(&config)
            .ok_or_else(|| eyre!("Could not load the wake-phrase model"))?;
        let stream = model.create_stream();
        Ok(Self { model, stream })
    }

    pub(super) fn reset(&mut self) {
        self.stream = self.model.create_stream();
    }

    pub(super) fn feed(&mut self, samples: &[f32], rate: u32) -> bool {
        self.stream.accept_waveform(rate as i32, samples);
        while self.model.is_ready(&self.stream) {
            self.model.decode(&self.stream);
            if self
                .model
                .get_result(&self.stream)
                .is_some_and(|r| !r.keyword.is_empty())
            {
                self.reset();
                return true;
            }
        }
        false
    }
}
