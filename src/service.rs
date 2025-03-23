use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use futures_core::Stream;
use tokio::sync::mpsc::Sender;
use tonic::{Request, Response, Status, Streaming};

use crate::proto::envoy::service::ext_proc::v3::external_processor_server::ExternalProcessor;
use crate::proto::envoy::service::ext_proc::v3::processing_request::Request::RequestHeaders;
use crate::proto::envoy::service::ext_proc::v3::{ProcessingRequest, ProcessingResponse};
use crate::storage::Storage;

#[derive(Clone)]
pub struct EnfireService {
    pub background_tasks: Sender<EnfireTask>,
    pub storage: Arc<dyn Storage + Send + Sync>,
}

#[tonic::async_trait]
impl ExternalProcessor for EnfireService {
    type ProcessStream = Pin<Box<dyn Stream<Item = Result<ProcessingResponse, Status>> + Send>>;

    async fn process(
        &self,
        request: Request<Streaming<ProcessingRequest>>,
    ) -> Result<Response<Self::ProcessStream>, Status> {
        let mut stream = request.into_inner();
        let task_channel = self.background_tasks.clone();
        let response = async_stream::try_stream! {
        while let Some(req) = stream.message().await? {
            if let Some(task) = EnfireTask::from_request(req.clone()) {
                let sender = task_channel.clone();

                // Fire and forget: send task to background
                tokio::spawn(async move {
                    if let Err(e) = sender.send(task).await {
                        eprintln!("Failed to send task: {}", e);
                    }
                });
            }

            yield ProcessingResponse {
                ..Default::default()
            };
        }};
        Ok(Response::new(Box::pin(response)))
    }
}

#[derive(Debug)]
pub struct EnfireTask {
    pub ip: String,
    pub path: String,
    pub key: Option<String>,
}

impl EnfireTask {
    pub fn from_request(message: ProcessingRequest) -> Option<Self> {
        if let Some(request) = message.request {
            let headers = match request {
                RequestHeaders(req) => {
                    let h = req
                        .headers
                        .and_then(|v| Some(v.headers))
                        .unwrap_or_else(|| vec![]);
                    HashMap::from_iter(h.into_iter().map(|i| (i.key, i.value)))
                }
                _ => HashMap::new(),
            };
            return Some(Self {
                ip: headers
                    .get("x-forwarded-for")
                    .and_then(|val| val.split(",").next())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                path: headers
                    .get(":path")
                    .map(|s| s.clone())
                    .unwrap_or_else(|| "/".into()),
                key: None, // TODO: use this for custom header key
            });
        };
        None
    }

    pub async fn run(self, storage: Arc<dyn Storage + Send + Sync>) {
        let _ = storage
            .record_request(&self.ip, &self.path, self.key.as_deref())
            .await;

        let is_abusive = storage
            .is_abusive(&self.ip, &self.path, self.key.as_deref())
            .await;

        if let Ok(true) = is_abusive {
            println!("[ABUSE] IP {} is abusive on {}", self.ip, self.path);
        } else {
            println!("[OKAY ] IP {} not abusive on {}", self.ip, self.path);
        }
    }
}
