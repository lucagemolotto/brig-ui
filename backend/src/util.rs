use reqwest::Client;
use tracing::info;
use axum::http::StatusCode;

pub async fn post_influx_query(query_body: String) -> Result<String, StatusCode> {
    info!("Executing flux query:\n{}", query_body);
    let host = "http://localhost:8086"; // InfluxDB v2 server
    let org = "SailingLab";
    let token="ijL6ry3VP0Hm5nAvP-wvHouC1l3ysIWty-VWCPgF7Bz-aKt-4Oi9zFMV_t8UkVnQSVwdxlRpdKjbAuPxx9umsA==";
    let client = Client::new();
    let query_result = client
        .post(format!("{}/api/v2/query?org={}", host, org))
        .header("Authorization", format!("Token {}", token))
        .header("Accept", "application/csv")
        .header("Content-Type", "application/vnd.flux")
        .body(query_body)
        .send()
        .await
        .map_err(|e| {
            info!("Request error: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let response_text = query_result.text().await.map_err(|e| {
        info!("Response text error: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(response_text)
}