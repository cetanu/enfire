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
    pub task_tx: Sender<EnfireTask>,
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

        let task_tx = self.task_tx.clone();

        let response = async_stream::try_stream! {
        while let Some(req) = stream.message().await? {
            if let Some(task) = EnfireTask::from_request(req.clone()) {
                println!("Processing task: {task:?}");
                let tx = task_tx.clone();

                // Fire and forget: send task to background
                tokio::spawn(async move {
                    if let Err(e) = tx.send(task).await {
                        eprintln!("Failed to send task: {}", e);
                    }
                });
            } else {
                eprintln!("Failed to parse request: {req:?}");
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
    pub async fn execute(self, storage: Arc<dyn Storage + Send + Sync>) {
        println!("Checking IP {} on {}...", self.ip, self.path);
        let _ = storage
            .record_request(&self.ip, &self.path, self.key.as_deref())
            .await;

        if let Ok(true) = storage
            .is_abusive(&self.ip, &self.path, self.key.as_deref())
            .await
        {
            println!("[ABUSE] IP {} is abusive on {}", self.ip, self.path);
        } else {
            println!("IP not abusive");
        }

        let _ = storage
            .save_event(format!(
                "IP={} PATH={} KEY={:?}",
                self.ip, self.path, self.key
            ))
            .await;
    }
}
