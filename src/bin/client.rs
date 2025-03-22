use std::collections::HashMap;

use enfire::proto::envoy::config::core::v3::{HeaderMap, HeaderValue};
use enfire::proto::envoy::service::ext_proc::v3::processing_request::Request;
use tokio_stream::StreamExt;

use enfire::proto::envoy::service::ext_proc::v3::external_processor_client::ExternalProcessorClient;
use enfire::proto::envoy::service::ext_proc::v3::{HttpHeaders, ProcessingRequest};

fn header(key: &str, value: &str) -> HeaderValue {
    HeaderValue {
        key: key.to_string(),
        value: value.to_string(),
        raw_value: format!("{}: {}", key, value).into(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ExternalProcessorClient::connect("http://[::1]:50051").await?;
    let stream = tokio_stream::iter(1..1000).map(|_| ProcessingRequest {
        request: Some(Request::RequestHeaders(HttpHeaders {
            headers: Some(HeaderMap {
                headers: vec![
                    header("x-forwarded-for", "10.50.52.12, 192.168.97.33"),
                    header(":path", "/login"),
                    header("x-foo-bar", "baz"),
                ],
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
