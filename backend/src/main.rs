//use axum::body::Bytes;
use axum::extract::Query;
use axum::http::Method;
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
mod util;
mod camera;

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

#[derive(Deserialize)]
struct ImageDataParams {
    camera: String,
    date: String,
    set: String,
    folder: String,
    img_num: String,
}
#[derive(Deserialize)]
struct CsvDataParams {
    start: String,
    end: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct ImageDataPoint{
    date: String,
    lat: Option<f64>,
    lon: Option<f64>,
    cog: Option<f64>,
    sog: Option<f64>,
    conductivity: Option<f64>,
    depth: Option<f64>,
    oxygen_percentage: Option<f64>,
    oxygen_ppm: Option<f64>,
    ph: Option<f64>,
    pressure: Option<f64>,
    salinity: Option<f64>,
    temperature: Option<f64>,
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

// pub async fn format_sd(host: &str) -> Result<Json<ReformatResponse>, StatusCode> {
    
//     let mut url = "";
//     if host == "cam1" {
//         url = "http://192.168.1.83/reformatsdcard";
//     } else if host == "cam2" {
//         url = "http://192.168.3.83/reformatsdcard";
//     }

//     let client = Client::new();

//     let request_body = ReformatRequest {
//         erase_all_data: true,
//     };

//     let response = client
//         .post(url)
//         .json(&request_body)
//         .send()
//         .await?;

//     Ok(Json(response))
// }

async fn status_call() -> Result<Json<ServiceStatus>, StatusCode> {
    let idronaut_status = check_service_status("IDRONAUT").await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let camera_capture_status = check_service_status("camera_capture").await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let status = ServiceStatus {
        idronaut: idronaut_status,
        camera_capture: camera_capture_status,
    };
    
    Ok(Json(status))

}

async fn image_data_call(Query(params): Query<ImageDataParams>) -> Result<Json<ImageDataPoint>, StatusCode> {
    let file = format!("/files/{}/{}/IMG_{}_1.tif", params.set, params.folder, params.img_num);
    let ts_query = format!(
        r#"from(bucket: "asv_data")
            |> range(start: {}T00:00:00Z, stop: {}T23:59:59Z)  // Replace with your date
            |> filter(fn: (r) => r._measurement == "micasense_data")
            |> filter(fn: (r) => r._field == "capture")
            |> filter(fn: (r) => r.camera == "{}")
            |> filter(fn: (r) => r._value == "{}")
            |> keep(columns: ["_time"])"#,
            params.date, params.date, params.camera, file
    );
    println!("Query 1:\n{}", ts_query);
    
    let ts_response: String = util::post_influx_query(ts_query).await?;
    if ts_response.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    let mut timestamp = String::new();
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(Cursor::new(ts_response));
        for result in reader.records() {
            println!("Result:\n{:?}", result);
            if let Ok(record) = result {
                if let Some(time) = record.get(3) { // 5 -> timestamp, 6..11 -> sensors
                    timestamp = time.to_string();
                }
            }
        }

    let idro_query = format!(
        r#"import "experimental"
from(bucket: "asv_data")
            |> range(start: experimental.addDuration(d: -1s, to: {}), stop: experimental.addDuration(d: 1s, to: {})) 
            |> filter(fn: (r) => r._measurement == "idronaut_data")
            |> pivot(rowKey:["_time"], columnKey: ["_field"], valueColumn: "_value")
            |> elapsed(unit: 1ns)
            |> sort(columns: ["elapsed"], desc: false)
            |> limit(n: 1)"#,
            timestamp, timestamp
    );

    println!("Query 2:\n{}", idro_query);
    let idro_response: String = util::post_influx_query(idro_query).await?;
    println!("Raw CSV Response:\n{}", idro_response);
    if idro_response.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut f_cond = None;
    let mut f_oxperc = None;
    let mut f_oxppm = None;
    let mut f_ph = None;
    let mut f_press = None;
    let mut f_sal = None;
    let mut f_temp = None;
    reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(Cursor::new(idro_response));
        for result in reader.records() {
            if let Ok(record) = result {
                if let (Some(cond), Some(oxperc), Some(oxppm),
                        Some(ph), Some(press), Some(sal), Some(temp)) = (record.get(7), record.get(8), record.get(9),
                                                                                                 record.get(10), record.get(11), record.get(12), record.get(13)) { // 5 -> timestamp, 6..11 -> sensors
                    if let Ok(parsed_cond) = cond.parse::<f64>(){
                        f_cond = Some(parsed_cond);
                    }
                    if let Ok(parsed_oxperc) = oxperc.parse::<f64>(){
                        f_oxperc = Some(parsed_oxperc);
                    }
                    if let Ok(parsed_oxppm) = oxppm.parse::<f64>(){
                        f_oxppm = Some(parsed_oxppm);
                    }
                    if let Ok(parsed_ph) = ph.parse::<f64>(){
                        f_ph = Some(parsed_ph);
                    }
                    if let Ok(parsed_press) = press.parse::<f64>(){
                        f_press = Some(parsed_press);
                    }
                    if let Ok(parsed_sal) = sal.parse::<f64>(){
                        f_sal = Some(parsed_sal);
                    }
                    if let Ok(parsed_temp) = temp.parse::<f64>(){
                        f_temp = Some(parsed_temp);
                    }
                }
            }
        }

    let gps_query = format!(
            r#"import "experimental"
    from(bucket: "asv_data")
                |> range(start: experimental.addDuration(d: -1s, to: {}), stop: experimental.addDuration(d: 1s, to: {})) 
                |> filter(fn: (r) => r._measurement == "gps_data2")
                |> pivot(rowKey:["_time"], columnKey: ["_field"], valueColumn: "_value")
                |> elapsed(unit: 1ns)
                |> sort(columns: ["elapsed"], desc: false)
                |> limit(n: 1)"#,
                timestamp, timestamp
    );
    let gps_response: String = util::post_influx_query(gps_query).await?;
    println!("Raw CSV Response:\n{}", gps_response);
    if gps_response.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut f_cog = None;
    let mut f_depth = None;
    let mut f_lat = None;
    let mut f_lon = None;
    let mut f_sog = None;
    reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(Cursor::new(gps_response));
        for result in reader.records() {
            if let Ok(record) = result {

                if let (Some(cog), Some(depth), Some(lat),
                        Some(lon), Some(sog)) = (record.get(7), record.get(8), record.get(9),
                                                             record.get(11), record.get(13)) {
                    if let Ok(parsed_cog) = cog.parse::<f64>(){
                        f_cog = Some(parsed_cog);
                    }
                    if let Ok(parsed_depth) = depth.parse::<f64>(){
                        f_depth = Some(parsed_depth);
                    }
                    if let Ok(parsed_lat) = lat.parse::<f64>(){
                        f_lat = Some(parsed_lat);
                    }
                    if let Ok(parsed_lon) = lon.parse::<f64>(){
                        f_lon = Some(parsed_lon);
                    }
                    if let Ok(parsed_sog) = sog.parse::<f64>(){
                        f_sog = Some(parsed_sog);
                    }
                }
            }
        }

    let datapoints= ImageDataPoint{
        date: timestamp,
        lat: f_lat,
        lon: f_lon,
        cog: f_cog,
        sog: f_sog,
        conductivity: f_cond,
        depth: f_depth,
        oxygen_percentage: f_oxperc,
        oxygen_ppm: f_oxppm,
        ph: f_ph,
        pressure: f_press,
        salinity: f_sal,
        temperature: f_temp,
    };
    Ok(Json(datapoints))
}

async fn get_csv_data(Query(params): Query<CsvDataParams>) -> Result<String, StatusCode>{
    let flux_query = format!(
        r#"from(bucket: "asv_data")
            |> range(start: {}:00Z, stop: {}:00Z)
            |> filter(fn: (r) => r["_measurement"] == "gps_data2" or r["_measurement"] == "idronaut_data" or r["_measurement"] == "micasense_data")
            |> drop(columns: ["_start", "_stop", "table", "result"])  
            |> yield(name: "last")"#,
        params.start, params.end
    );

    match util::post_influx_query(flux_query).await{
        Ok(res) => return Ok(res),
        Err(e) => return Err(e)
    }
}

#[tokio::main]
async fn main() {
    // initialize logging.
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/api/data", get(query_data))
        .route("/api/status", get(status_call))
        .route("/api/camera_status", get(camera::camera_status_call))
        .route("/api/camera_folders", get(camera::camera_folders_call))
        .route("/api/image_data", get(image_data_call))
        .route("/api/reformat/:host", get(status_call))
        .route("/api/:service/:action", post(service_call))
        .route("/api/get_last_capture", get(camera::get_last_capture))
        .route("/api/download_data", get(get_csv_data))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(Any)
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server running on {}", addr);

    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}