use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::extract::State;
use axum::response::IntoResponse;
use logging::logging_service_client::LoggingServiceClient;
use logging::{GetLogsRequest, Log};
use reqwest::StatusCode;
use uuid::Uuid;

pub mod logging {
    tonic::include_proto!("logging");
}

use axum::{
    extract::Query,
    routing::{get, post},
    Router,
};

#[derive(Clone)]
struct Client {
    client: Arc<Mutex<LoggingServiceClient<tonic::transport::Channel>>>,
}

async fn add_log(
    State(client): State<Client>,
    Query(log): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut client = client.client.lock().await;
    let uuid = Uuid::new_v4().to_string();
    let message = log.get("message").unwrap().to_string();

    let request = tonic::Request::new(Log { uuid, message });

    client.add_log(request).await.unwrap();
    StatusCode::OK
}

async fn get_logs(State(client): State<Client>) -> impl IntoResponse {
    let mut client = client.client.lock().await;
    let request = tonic::Request::new(GetLogsRequest {});
    client.get_logs(request).await.unwrap();
    StatusCode::OK
}

#[tokio::main]
async fn main() {
    let client = Client {
        client: Arc::new(Mutex::new(
            LoggingServiceClient::connect("http://localhost:13228")
                .await
                .expect("Failed to connect to logging service"),
        )),
    };
    let app = Router::new()
        .route("/", get(get_logs))
        .with_state(client.clone())
        .route("/", post(add_log))
        .with_state(client);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:13226")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let mut client = LoggingServiceClient::connect("http://localhost:13228").await?;
//
//     let request = tonic::Request::new(Log {
//         uuid: Uuid::new_v4().to_string(),
//         message: "Test message".to_string(),
//     });
//
//     client.add_log(request).await?;
//     let request = tonic::Request::new(Log {
//         uuid: Uuid::new_v4().to_string(),
//         message: "Test message".to_string(),
//     });
//     client.add_log(request).await?;
//
//     let request = tonic::Request::new(GetLogsRequest {});
//     let response = client.get_logs(request).await?.into_inner().logs_string;
//     println!("{}", response);
//     Ok(())
// }
