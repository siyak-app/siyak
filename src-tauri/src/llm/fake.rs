//! A provider for tests: answers every request with fixed pieces of text and
//! records the requests it received.

use std::sync::{Arc, Mutex};

use reqwest::Url;
use tokio_util::sync::CancellationToken;

use super::{
    Completion, Finish, LlmFuture, LlmProvider, LlmRequest, ModelInfo, ProviderFactory, TextSink,
    Usage,
};
use crate::error::{AppError, AppResult};
use crate::secrets::ProviderId;

/// Answers with `pieces`, or fails with `error`.
#[derive(Default)]
pub struct FakeProvider {
    /// The pieces of every answer.
    pub pieces: Vec<String>,
    /// The usage reported; `None` reports none.
    pub usage: Option<Usage>,
    /// Fails every request with this error instead.
    pub error: Option<AppError>,
    /// Every request received.
    pub seen: Mutex<Vec<LlmRequest>>,
}

impl FakeProvider {
    /// A provider answering with `pieces` and reporting `usage`.
    pub fn answering(pieces: &[&str], usage: Usage) -> Arc<Self> {
        Arc::new(Self {
            pieces: pieces.iter().map(|p| (*p).to_owned()).collect(),
            usage: Some(usage),
            ..Self::default()
        })
    }

    /// The requests received so far.
    pub fn requests(&self) -> Vec<LlmRequest> {
        self.seen.lock().unwrap().clone()
    }
}

impl LlmProvider for FakeProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Anthropic
    }

    fn list_models(&self) -> LlmFuture<'_, Vec<ModelInfo>> {
        Box::pin(async {
            Ok(vec![ModelInfo {
                id: "fake-model".into(),
                name: None,
            }])
        })
    }

    fn stream<'a>(
        &'a self,
        request: &'a LlmRequest,
        on_text: TextSink<'a>,
        cancel: &'a CancellationToken,
    ) -> LlmFuture<'a, Completion> {
        Box::pin(async move {
            self.seen.lock().unwrap().push(request.clone());
            if let Some(err) = &self.error {
                return Err(err.clone());
            }
            for piece in &self.pieces {
                if cancel.is_cancelled() {
                    return Ok(Completion {
                        usage: None,
                        finish: Finish::Cancelled,
                    });
                }
                on_text(piece);
            }
            Ok(Completion {
                usage: self.usage,
                finish: Finish::Complete,
            })
        })
    }
}

/// Hands out one provider whatever is asked for.
pub struct FakeFactory(pub Arc<FakeProvider>);

impl ProviderFactory for FakeFactory {
    fn create(&self, _id: ProviderId, _server: Option<Url>) -> AppResult<Arc<dyn LlmProvider>> {
        Ok(Arc::clone(&self.0) as Arc<dyn LlmProvider>)
    }
}
