use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task;
use tonic::transport::Server;

use enfire::proto::envoy::service::ext_proc::v3::external_processor_server::ExternalProcessorServer;
use enfire::service::{EnfireService, EnfireTask};
use enfire::storage::SqliteStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let (tx, mut rx) = mpsc::channel::<EnfireTask>(100);

    let storage = Arc::new(SqliteStorage::new("data/enfire.db").await?);
    let service = EnfireService {
        task_tx: tx.clone(),
        storage: storage.clone(),
    };

    // Background task processor
    task::spawn({
        let storage = storage.clone();
        async move {
            while let Some(task) = rx.recv().await {
                task.execute(storage.clone()).await;
            }
        }
    });

    println!("Enfire ExternalProcessor listening on {}", addr);
    Server::builder()
        .add_service(ExternalProcessorServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
