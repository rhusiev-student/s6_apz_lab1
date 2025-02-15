use logging::logging_service_server::{LoggingService, LoggingServiceServer};
use logging::{AddLogResponse, GetLogsRequest, Log, LogsString};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tonic::{transport::Server, Request, Response, Status};

mod logging {
    tonic::include_proto!("logging");
}

#[derive(Debug, Default)]
pub struct Logger {
    logs: Arc<Mutex<HashMap<String, Log>>>,
}

#[tonic::async_trait]
impl LoggingService for Logger {
    async fn add_log(&self, request: Request<Log>) -> Result<Response<AddLogResponse>, Status> {
        let mut logs = match self.logs.lock() {
            Ok(logs) => logs,
            Err(_) => {
                println!("Got a request to add log, but failed to access logs");
                return Err(Status::internal("Failed to add log"));
            }
        };
        let log = request.into_inner();
        println!("Got a request to add log: ({:?}, {:?})", log.uuid, log.message);

        logs.insert(log.uuid.clone(), log);

        Ok(Response::new(AddLogResponse { success: true }))
    }
    async fn get_logs(
        &self,
        request: Request<GetLogsRequest>,
    ) -> Result<Response<LogsString>, Status> {
        print!("Got a request to get logs: ");
        let logs = match self.logs.lock() {
            Ok(logs) => logs,
            Err(_) => {
                println!("Failed to access logs for a request: {:?}", request);
                return Err(Status::internal("Failed to get logs"));
            }
        };
        println!("Success");
        Ok(Response::new(LogsString {
            logs_string: logs
                .values()
                .map(|log| &log.message)
                .fold(String::new(), |acc, message| acc + &message + "\n"),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "0.0.0.0:13228".parse()?;
    let logger = Logger {
        logs: Arc::new(Mutex::new(HashMap::new())),
    };

    Server::builder()
        .add_service(LoggingServiceServer::new(logger))
        .serve(address)
        .await?;

    Ok(())
}
