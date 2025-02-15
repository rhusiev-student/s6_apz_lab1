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
    logging: Arc<Mutex<LoggingServiceClient<tonic::transport::Channel>>>,
    message: Arc<Mutex<reqwest::Client>>,
    message_url: String,
}

async fn add_log(
    State(client): State<Client>,
    Query(log): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut client = client.logging.lock().await;
    let uuid = Uuid::new_v4().to_string();
    let message = match log.get("message") {
        Some(msg) => msg.to_string(),
        None => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let request = tonic::Request::new(Log { uuid, message });

    match client.add_log(request).await {
        Ok(_) => StatusCode::OK,
        Err(err) => {
            println!("Error while adding log: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn get_logs(State(client): State<Client>) -> impl IntoResponse {
    let message = match client
        .message
        .lock()
        .await
        .get(&client.message_url)
        .send()
        .await
    {
        Ok(response) => match response.text().await {
            Ok(msg) => msg,
            Err(err) => {
                println!("Error while getting a message request: {}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
        },
        Err(err) => {
            println!("Error while creating a message request: {}", err);
            return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
        }
    };

    let mut client = client.logging.lock().await;
    let logs = tonic::Request::new(GetLogsRequest {});

    match client.get_logs(logs).await {
        Ok(response) => (
            StatusCode::OK,
            (message + "\n" + &response.into_inner().logs_string),
        )
            .into_response(),
        Err(err) => {
            println!("Error while getting logs: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, err.message().to_string()).into_response()
        }
    }
}

#[tokio::main]
async fn main() {
    let client = Client {
        logging: Arc::new(Mutex::new(
            LoggingServiceClient::connect("http://localhost:13228")
                .await
                .expect("Failed to connect to logging service"),
        )),
        message: Arc::new(Mutex::new(reqwest::Client::new())),
        message_url: "http://localhost:13227".to_string(),
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
