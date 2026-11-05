//! The local backend: ONNX Runtime (`ort`) + `tokenizers`, configured from the
//! model pack's metadata ([`super::space::ModelPackSpec`]: prefixes, pooling,
//! token limit). The model loads on first use; inference runs on a blocking
//! thread (spec §7.4).
//!
//! TODO(M7): `OnnxEmbedder`, once the model is chosen (spec §18).
