use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::process::Command;
use tracing::info;
use axum::{
    Json,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use csv::ReaderBuilder;
use std::io::Cursor;
use chrono::{DateTime, Utc};
use tower_http::cors::{CorsLayer, Any};
use tokio::task;

#[derive(Debug, Serialize, Deserialize)]
struct DataPoint {
    time: String,
    field: String,
    value: f64,
    epochtime: f64, 
}

impl DataPoint {
    fn from_raw(time: &str, value: f64, field: &str) -> Option<Self> {
        if let Ok(dt) = time.parse::<DateTime<Utc>>() {
            Some(Self {
                time: time.to_string(),                // Keep original string
                epochtime: dt.timestamp_millis() as f64, // Convert to milliseconds since epoch
                value,
                field: field.to_string(),
            })
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ServiceStatus {
    idronaut: bool,
    camera_capture: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServiceStatus {
    cam1_free: f32,
    cam1_total: f32,
    cam2_free: f32,
    cam2_total: f32,
}

// queries influxdb for idronaut data
async fn query_data() -> Result<Json<Vec<DataPoint>>, StatusCode> {
    let host = "http://localhost:8086"; // InfluxDB v2 server
    let org = "SailingLab";
    let token="ijL6ry3VP0Hm5nAvP-wvHouC1l3ysIWty-VWCPgF7Bz-aKt-4Oi9zFMV_t8UkVnQSVwdxlRpdKjbAuPxx9umsA==";
    let bucket = "asv_data";

    let query = format!(
        "from(bucket: \"{}\")
        |> range(start: -24h)
        |> filter(fn: (r) => r[\"_measurement\"] == \"idronaut_data\")
        |> filter(fn: (r) => r[\"_field\"] == \"conductivity\" 
        or r[\"_field\"] == \"oxygen_percentage\" 
        or r[\"_field\"] == \"oxygen_ppm\" 
        or r[\"_field\"] == \"ph\" 
        or r[\"_field\"] == \"pressure\" 
        or r[\"_field\"] == \"temperature\" 
        or r[\"_field\"] == \"salinity\")
        |> aggregateWindow(every: 5m, fn: mean, createEmpty: false)
        |> yield()",
        bucket
    );

    let client = Client::new();
    let response = client
        .post(format!("{}/api/v2/query?org={}", host, org))
        .header("Authorization", format!("Token {}", token))
        .header("Accept", "application/csv")
        .header("Content-Type", "application/vnd.flux")
        .body(query)
        .send()
        .await
        .map_err(|e| {
            info!("Request error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    //println!("{:?}", responsee);
    let response_text = response.text().await.map_err(|e| {
        info!("Response text error: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    println!("Raw CSV Response:\n{}", response_text);  // debug

    let mut data_points = Vec::new();
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(Cursor::new(response_text));

        for result in reader.records() {
            if let Ok(record) = result {
                if let (Some(time), Some(value), Some(field)) = (record.get(5), record.get(6), record.get(7)) { // 5 -> timestamp, 6..11 -> sensors
                    if let Ok(parsed_value) = value.parse::<f64>() {
                        if let Some(data_point) = DataPoint::from_raw(time, parsed_value, field) {
                            data_points.push(data_point);
                        }
                    }
                }
            }
        }
    println!("Sent data:\n{:?}", data_points);
    Ok(Json(data_points))
}

// start/stop services
async fn service_call(Path((service, action)): Path<(String, String)>) -> impl IntoResponse {
    let valid_services = ["camera_capture", "IDRONAUT"];
    let valid_actions = ["start", "stop"];
    // sanitize input
    if !valid_services.contains(&service.as_str()) || !valid_actions.contains(&action.as_str()) {
        return (StatusCode::BAD_REQUEST, "Invalid service or action").into_response();
    }
    //let msg = format!("Service {} has been {}ed", service, action);
    //println!("Service {} has been {}ed", service, action);
    match control_service(&service, &action).await {
        Ok(msg) => 
            {println!("Service {} has been {}ed", service, action);
            (StatusCode::OK, msg).into_response()},
        Err(err) =>
            {println!("Service {} has could not be {}ed", service, action);
            (StatusCode::INTERNAL_SERVER_ERROR, err).into_response()},
    }
}

// uses action on systemctl service named {service}
// returns systemctl std out
async fn control_service(service: &str, action: &str) -> Result<String, String> {
    let service_owned = service.to_owned();
    let action_owned = action.to_owned();

    // spawn blocking task to run the system command.
    let result = task::spawn_blocking(move || {
        Command::new("systemctl")
            .arg(action_owned)
            .arg(service_owned)
            .output()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    match result {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        Err(e) => Err(format!("Failed to run command: {}", e)),
    }
}


// get services status
async fn service_status() -> impl IntoResponse {
    
    let msg = format!("serv ok");
    //println!("Service {} has been {}ed", service, action);
    (StatusCode::OK, msg).into_response()
}

// runs systemctl is-active and returns true if active
async fn check_service_status(service: &str) -> Result<bool, String> {
    let service_owned = service.to_owned();
    let result = task::spawn_blocking(move || {
        Command::new("systemctl")
            .arg("is-active")
            .arg(&service_owned)
            .output()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;
    
    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim() == "active")
        }
        Err(e) => Err(format!("Command error: {}", e)),
    }
}

// queries systemctl for idronaut and camera_capture services
async fn status_call() -> Result<Json<ServiceStatus>, StatusCode> {
    let idronaut_status = check_service_status("IDRONAUT").await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let camera_capture_status = check_service_status("camera_capture").await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    // camera 1 (red)
    match client.get("http://192.168.1.83:80/status").send().await {
        Ok(response) => match response.json::<Vec<DataPoint>>().await {
            Ok(data) => res = data,
            Err(_) => res = vec![],
        },
        Err(_) => res = vec![],
    }
    let status = CamStatus {
        cam1_free: idronaut_status,
        cam1_total: ,
        cam2_free: ,
        cam2_total: ,
    };
    
    Ok(Json(status))
}

async fn status_call() -> Result<Json<ServiceStatus>, StatusCode> {
    let idronaut_status = check_service_status("IDRONAUT").await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let camera_capture_status = check_service_status("camera_capture").await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let status = ServiceStatus {
        idronaut: idronaut_status,
        camera_capture: camera_capture_status,
    };
    
    Ok(Json(status))
}

#[tokio::main]
async fn main() {
    // initialize logging.
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/api/data", get(query_data))
        .route("/api/status", get(status_call))
        .route("/api/:service/:action", post(service_call))
        .layer(CorsLayer::new().allow_origin(Any)); // needed for cors policy

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server running on {}", addr);

    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}