# Wake-phrase model

These are the int8 inference weights, token vocabulary, and SentencePiece tokenizer from [`sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01`](https://k2-fsa.github.io/sherpa/onnx/kws/pretrained_models/index.html), downloaded from the [official sherpa-onnx model release](https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2).

The model is a 3.3M-parameter English Zipformer trained on GigaSpeech XL. Its published model card identifies the license as Apache-2.0. The original model card is available in the release archive and [ModelScope repository](https://www.modelscope.cn/pkufool/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01). See [LICENSE](LICENSE).

Only the wake phrase uses these weights. Russian or English commands are transcribed using Hex's selected speech model after activation. The files are embedded in the app, so enabling voice control needs no model download, Python installation, service, or cloud account. They are materialized in the fork's data folder when the opt-in worker starts.

Pinned SHA-256:

| File | SHA-256 |
| --- | --- |
| encoder.onnx | `1e721676515bcd42a186979733981213c66c80db680e1cc582dfedf3be76e678` |
| decoder.onnx | `e40ff43297abe815e8898494c17e71bba2152d9d40fa3eb803f75d0f7533329a` |
| joiner.onnx | `eae9da0c7e1e6c6a3f4cc42d167899c388f6c6701b94cb96320e4f55df79624c` |
| tokens.txt | `fd2ded4050a55d2b1578870ba8697d02371980217806b7558bd0a5cc60f3ba53` |
| bpe.model | `c8a2a0129c4ab8e463164c142f82d25649661b122c8cd0b7aab5c9e80b90ad24` |
