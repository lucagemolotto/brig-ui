use reqwest::Client;
use axum::{
    extract::Query, http::StatusCode, Json, body::Bytes};
use serde::{Serialize, Deserialize};
use csv::ReaderBuilder;
use std::io::Cursor;
use tracing::info;
use std::collections::HashSet;
use image::codecs::jpeg::JpegEncoder;
use image::{load_from_memory_with_format, ImageFormat};


use crate::util;

#[derive(Debug, Deserialize)]
struct SvInfo {
    azimuth: Option<f64>,
    channel: Option<i32>,
    cno: Option<i32>,
    diff_flag: Option<bool>,
    elevation: Option<f64>,
    orbit_info: Option<bool>,
    orbit_is_eph: Option<bool>,
    quality: Option<i32>,
    sv_healthy: Option<bool>,
    svid: Option<i32>,
    used_flag: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RedEdgeStatus {
    sd_gb_free: Option<f64>,
    sd_gb_total: Option<f64>,
    sd_gb_type: Option<String>,
    sd_warn: Option<bool>,
    sd_status: Option<String>,
    bus_volts: Option<f64>,
    gps_used_sats: Option<i32>,
    gps_vis_sats: Option<i32>,
    gps_warn: Option<bool>,
    gps_lat: Option<f64>,
    gps_lon: Option<f64>,
    gps_type: Option<String>,
    course: Option<f64>,
    alt_agl: Option<f64>,
    alt_msl: Option<f64>,
    p_acc: Option<f64>,
    utc_time: Option<String>,
    vel_2d: Option<f64>,
    sv_info: Option<Vec<SvInfo>>,
    auto_cap_active: Option<bool>,
    dls_status: Option<String>,
    gps_time: Option<String>,
    utc_time_valid: Option<bool>,
    time_source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CameraSpace {
    cam1_free: f64,
    cam1_total: f64,
    cam2_free: f64,
    cam2_total: f64,
}

#[derive(Deserialize)]
pub struct CameraFoldersParams {
    camera: String,
    date: String,
}

#[derive(Deserialize)]
pub struct FormatParams {
    camera: String,
}

#[derive(Deserialize)]
pub struct CaptureParams {
    cam: String,
    band: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReformatRequest{
    erase_all_data: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReformatResponse{
    message: String,
    reformat_status: String,
}

// queries RedEdge HTTP APIs for camera status
pub async fn camera_status_call() -> Result<Json<CameraSpace>, StatusCode> {
    let client: Client = Client::new();
    
    let mut cam1_fr = 0.0;
    let mut cam1_tot = 0.0;
    let mut cam2_fr = 0.0;
    let mut cam2_tot = 0.0;

    // camera 1 (red)
    match client.get("http://192.168.1.83:80/status").send().await {
        Ok(response) => match response.json::<RedEdgeStatus>().await {
            Ok(data) => {
                cam1_fr = data.sd_gb_free.unwrap();
                cam1_tot = data.sd_gb_total.unwrap();
            },
            Err(_) => {
                cam1_fr = -1.0;
                cam1_tot =-1.0;
            },
        },
        Err(_) => {
            cam1_fr = -1.0;
            cam1_tot =-1.0;
        },
    }

    // camera 2 (blue)
    match client.get("http://192.168.3.83:80/status").send().await {
        Ok(response) => match response.json::<RedEdgeStatus>().await {
            Ok(data) => {
                cam2_fr = data.sd_gb_free.unwrap();
                cam2_tot = data.sd_gb_total.unwrap();
            },
            Err(_) => {
                cam2_fr = -1.0;
                cam2_tot =-1.0;
            },
        },
        Err(_) => {
            cam2_fr = -1.0;
            cam2_tot =-1.0;
        },
    }
    let status = CameraSpace {
        cam1_free: cam1_fr,
        cam1_total: cam1_tot,
        cam2_free: cam2_fr,
        cam2_total: cam2_tot,
    };
    
    Ok(Json(status))
}

// queries Influx for captures of given camera on a certain date, returns the folder generated on said date
pub async fn camera_folders_call(Query(params): Query<CameraFoldersParams>) -> Result<Json<Vec<String>>, StatusCode> {
    let camera = params.camera;
    let req_date = params.date;
    
    // Format the date strings for the Flux query
    let start_time = format!("{}T00:00:00Z", req_date);
    let end_time = format!("{}T23:59:59Z", req_date);

    // Build the Flux query
    let flux_query = format!(
        r#"from(bucket: "asv_data")
          |> range(start: {}, stop: {})
          |> filter(fn: (r) => r._measurement == "micasense_data")
          |> filter(fn: (r) => r._field == "capture")
          |> filter(fn: (r) => r.camera == "{}")"#,
        start_time, end_time, camera
    );

    let response_text: String = util::post_influx_query(flux_query).await?;
    if response_text.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    println!("Raw CSV Response:\n{}", response_text);
    let mut data_points = Vec::new();
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(Cursor::new(response_text));

        for result in reader.records() {
            if let Ok(record) = result {
                if let Some(set) = record.get(6) {
                    if let Some(info) = extract_set_info(set) {
                        data_points.push(info);
                    } else {
                        println!("No set info could be extracted from: {}", set);
                    }
                }
            }
        }
    let unique_data_points: Vec<String> = data_points.into_iter().collect::<HashSet<_>>().into_iter().collect();
    println!("Sent data:\n{:?}", unique_data_points);
    Ok(Json(unique_data_points))

}
// Extracts folder and subfolder from influx value
fn extract_set_info(tag: &str) -> Option<String> {
    // Extract /SETXXXX/YYY from files/SETXXXX/YYY/IMG_ZZZZ.tif
    let parts: Vec<&str> = tag.split('/').collect();
    if parts.len() >= 3 {
        // Return the SET and directory part (/SETXXXX/YYY)
        return Some(format!("{}/{}", parts[2], parts[3]));
    }
    None
}

// Given a camera and a band, returns (if it was taken in the last hour) the JPEG bytes of the last capture in the given band by the camera
pub async fn get_last_capture(Query(params): Query<CaptureParams>) -> Result<Bytes, StatusCode>{
    if (params.cam != "cam1") && (params.cam != "cam2"){
        return Err(StatusCode::NOT_FOUND)
    }
    let mut filename = get_last_capture_filename(&params.cam).await?;
    let mut cam_url = "192.168.1.83";
    if params.cam == "cam2" {
        cam_url = "192.168.3.83";
    }
    if filename == "" {
        return Err(StatusCode::NOT_FOUND)
    }
    
    match params.band.parse::<i32>(){
        Ok(num) => {
            if num < 1 || num > 5 {
                return Err(StatusCode::NOT_FOUND);
            } else if num > 1 && num <= 5 {
                filename.truncate(filename.len() - 5);
                filename = format!("{}{}.tif", filename, num);
            }
        },
        Err(_) => return Err(StatusCode::NOT_FOUND)
    }
    let micasense_url = format!("http://{}{}", cam_url, filename);
    let client = Client::new();
    println!("url: {}", micasense_url);
    
    match client.get(&micasense_url).send().await {
        Ok(response) => {
            // Check if the response was successful
            if response.status().is_success() {
                match response.bytes().await {
                    Ok(bytes) => {
                        // Convert TIF to JPEG
                        match convert_tif_to_jpeg(&bytes) {
                            Ok(jpeg_bytes) => {
                                return Ok(jpeg_bytes);
                            }
                            Err(_) => {
                                // Error during conversion
                                return Err(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                    }
                    Err(_) => {
                        return Err(StatusCode::NOT_FOUND);
                    }
                }
            } else {
                return Err(StatusCode::NOT_FOUND);
            }
        }
        Err(_) => {
            return Err(StatusCode::NOT_FOUND);
        }
    }
    //Ok(filename_1)
}

// Queries InfluxDB for the last capture's filename in the last hour of a given camera
async fn get_last_capture_filename(camera: &str) -> Result<String, StatusCode> {
    let flux_query = format!(
        r#"from(bucket: "asv_data")
            |> range(start: -1h)
            |> filter(fn: (r) => r._measurement == "micasense_data")
            |> filter(fn: (r) => r._field == "capture")
            |> filter(fn: (r) => r.camera == "{}")
            |> last()
            |> yield(name: "last")"#,
        camera
    );
    let response: String = util::post_influx_query(flux_query).await?;
    println!("Raw CSV Response:\n{}", response);
    if response.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    let mut res = "".to_string();
    let mut reader = ReaderBuilder::new()
    .has_headers(true)
    .from_reader(Cursor::new(response));
    for result in reader.records() {
        if let Ok(record) = result {
            if let Some(str) = record.get(6){
                res = str.to_string();
            }
        }
    }
    Ok(res)
}


// Converts TIFF image bytes to JPEG bytes
fn convert_tif_to_jpeg(tif_bytes: &[u8]) -> Result<axum::body::Bytes, image::ImageError> {
    //println!("loading bytes...");
    // load the TIF image from bytes
    let img_res = load_from_memory_with_format(tif_bytes, ImageFormat::Tiff);
    //let mut jpeg_buffer = Vec::new();
    match img_res {
        Ok(img) => {
            println!("converting...");
            // write the image to the buffer in JPEG format
            let mut default = vec![];
            let encoder = JpegEncoder::new(&mut default);
            match img.to_rgb8().write_with_encoder(encoder) {
                Ok(_) => 
                println!("all ok"),
                Err(e) => println!("error: {:?}", e)
            }
            // match img.write_to(&mut Cursor::new(&mut jpeg_buffer), ImageFormat::Jpeg) {
            //     Ok(_) => println!("all ok"),
            //     Err(e) => println!("error: {:?}", e)
            // }
            
            // convert the buffer to bytes
            Ok(axum::body::Bytes::from(default))
        }
        Err (e) => {
            println!("loading error: {:?}", e);
            return Err(e)
        }
    }
    
    // Create a buffer to store the JPEG image
    
}

pub async fn format_sd(Query(params): Query<FormatParams>) -> Result<Json<ReformatResponse>, StatusCode> {
    
    let mut url = "";
    if params.camera == "cam1" {
        url = "http://192.168.1.83/reformatsdcard";
    } else if params.camera == "cam2" {
        url = "http://192.168.3.83/reformatsdcard";
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let client = Client::new();

    let request_body = ReformatRequest {
        erase_all_data: true,
    };

    let response = client
        .post(url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            info!("Request error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let resp: Result<ReformatResponse, reqwest::Error> = response.json().await;
    match resp {
        Ok(res) => return Ok(Json(res)),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}