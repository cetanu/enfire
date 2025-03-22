use std::pin::Pin;
use std::sync::Arc;

use futures_core::Stream;
use tokio::sync::mpsc::Sender;
use tonic::{Request, Response, Status, Streaming};

use crate::proto::envoy::service::ext_proc::v3::external_processor_server::ExternalProcessor;
use crate::proto::envoy::service::ext_proc::v3::processing_request::Request::{
    RequestBody, RequestHeaders, ResponseBody, ResponseHeaders,
};
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
            let task = EnfireTask::from_request(req.clone());
            let tx = task_tx.clone();

            // Fire and forget: send task to background
            tokio::spawn(async move {
                if let Err(e) = tx.send(task).await {
                    eprintln!("Failed to send task: {}", e);
                }
            });

            yield ProcessingResponse {
                ..Default::default()
            };
        }};
        Ok(Response::new(Box::pin(response)))
    }
}

#[derive(Debug)]
pub struct EnfireTask {
    pub detail: String,
}

impl EnfireTask {
    pub fn from_request(message: ProcessingRequest) -> Self {
        if let Some(request) = message.request {
            match request {
                RequestHeaders(req) => {
                    for header in req.headers.unwrap().headers {
                        println!("Received= {}: {}", header.key, header.value);
                    }
                }
                ResponseHeaders(req) => todo!(),
                RequestBody(req) => todo!(),
                ResponseBody(req) => todo!(),
                _ => todo!(),
            };
        };
        Self {
            detail: "foo".into(),
        }
    }

    pub async fn execute(self, storage: Arc<dyn Storage + Send + Sync>) {
        println!("Executing background task: {:?}", self.detail);
        let _ = storage.save_event(self.detail).await;
    }
}
