use std::collections::HashMap;

use enfire::proto::envoy::config::core::v3::{HeaderMap, HeaderValue};
use enfire::proto::envoy::service::ext_proc::v3::processing_request::Request;
use tokio_stream::StreamExt;

use enfire::proto::envoy::service::ext_proc::v3::external_processor_client::ExternalProcessorClient;
use enfire::proto::envoy::service::ext_proc::v3::{HttpHeaders, ProcessingRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ExternalProcessorClient::connect("http://[::1]:50051").await?;
    let stream = tokio_stream::iter(1..10).map(|_| ProcessingRequest {
        request: Some(Request::RequestHeaders(HttpHeaders {
            headers: Some(HeaderMap {
                headers: vec![HeaderValue {
                    key: "foo".to_string(),
                    value: "bar".to_string(),
                    raw_value: "foo: bar".into(),
                }],
            }),
            ..Default::default()
        })),
        attributes: HashMap::new(),
        metadata_context: None,
        observability_mode: false,
    });
    let result = client.process(stream).await?;
    println!("{result:?}");
    Ok(())
}
