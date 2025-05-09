use leptos::*;
use leptos_router::components::{Router, Route, Routes, A};
use leptos_router::path;
use reqwest::Client;
use gloo::timers::callback::Interval;
use serde::Deserialize;
use leptos_chartistry::*;
use leptos::suspense::Suspense;
use leptos::prelude::*;
use leptos::task::spawn_local;
use chrono::{DateTime, FixedOffset};
use web_sys::console;
use tracing::info;
use const_format::concatcp;  

mod camera;
mod datavis;
mod util;
#[derive(Deserialize, Clone, Debug)]
struct DataPoint {
    time: String,
    value: f64,
    field: String,
    epochtime: f64,
}


#[derive(serde::Deserialize, Clone, Debug)]
struct ServiceStatus {
    idronaut: bool,
    camera_capture: bool,
}

const BASEURL: &'static str = "http://192.168.2.9:3000";

//#[tokio::main]
fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Some info");
    leptos::mount::mount_to_body(App);
}

// main component
#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <SiteHeader />
            <main>
                <Routes fallback=|| "Not found.">
                    <Route path=path!("/") view=Home/>
                    <Route path=path!("/Charts/") view=Home/>
                    <Route path=path!("/Status/") view=Status/>
                    <Route path=path!("/Cameras/") view=camera::camera_page/>
                    <Route path=path!("/Data/") view=datavis::data_page/>
                </Routes>
            </main>
        </Router>
    }
}

// queries backend for sensor data, atm only asks for idronaut data
async fn load_data(client: Client) -> Vec<DataPoint> {
    info!("Loading data...");
    let mut res = vec![];
    match client.get(concatcp!(BASEURL, "/api/data")).send().await {
        Ok(response) => match response.json::<Vec<DataPoint>>().await {
            Ok(data) => res = data,
            Err(_) => res = vec![],
        },
        Err(_) => res = vec![],
    }
    info!("Data received: {:?}", res);
    res
}

// post for service calls
async fn service_request(client: Client, name: &str, action: &str, result: &RwSignal<Option<String>>, popup: &RwSignal<bool>) -> () {
    let addr = format!("/api/{}/{}", name, action).to_owned();
    let mut url : String = "".to_owned();
    url.push_str(BASEURL);
    url.push_str(&addr);
    match client.post(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let res = format!("{} {} request sent successfully", action, name);
                info!(res);
                result.set(Some(res));
                popup.set(true);
            } else {
                let res = format!("Failed to {} {}", action, name);
                info!(res);
                result.set(Some(res));
                popup.set(true);
            }
        }
        Err(err) => {
            let res = format!("Error sending request: {:?}", err);
            info!(res);
            result.set(Some(res));
            popup.set(true);
            }
    }
}

// component displaying chart for given series and data
#[component]
fn SensorChart(title: String, data: RwSignal<Vec<DataPoint>>, series: Series<DataPoint, DateTime<FixedOffset>,  f64>) -> impl IntoView {
    view! {
        <div class="chart-container">
            <Chart
                aspect_ratio=AspectRatio::from_outer_height(300.0, 1.2)
                series=series
                data=data
                top=RotatedLabel::middle(title)
                left=TickLabels::aligned_floats()
                bottom=TickLabels::timestamps()
                inner=[
                    AxisMarker::left_edge().into_inner(),
                    AxisMarker::bottom_edge().into_inner(),
                    XGridLine::default().into_inner(),
                    YGridLine::default().into_inner(),
                    YGuideLine::over_mouse().into_inner(),
                    XGuideLine::over_data().into_inner(),
                ]
                tooltip=Tooltip::left_cursor().show_x_ticks(true)
            />
        </div>
    }
}

// home component, displays charts and service monitoring
#[component]
fn Home() -> impl IntoView {

    let client = Client::new();
    let data = LocalResource::new(move || {
        let client = client.clone();
        async move { 
            load_data(client).await
        }
    });

    //let chart_data = RwSignal::new(vec![]);
    //let chart_data2 = RwSignal::new(vec![]);
    let temperature_chart_data = RwSignal::new(vec![]);
    let pressure_chart_data = RwSignal::new(vec![]);
    let o2_perc_chart_data = RwSignal::new(vec![]);
    let o2_pmm_chart_data = RwSignal::new(vec![]);
    let salinity_chart_data = RwSignal::new(vec![]);
    let conductivity_chart_data = RwSignal::new(vec![]);
    let ph_chart_data = RwSignal::new(vec![]);

    // Effect to update chart data
    Effect::new(move || {
        if let Some(points) = data.get() {
            temperature_chart_data.set(points.iter().filter(|p| p.field == "temperature").cloned().collect());
            pressure_chart_data.set(points.iter().filter(|p| p.field == "pressure").cloned().collect());
            o2_pmm_chart_data.set(points.iter().filter(|p| p.field == "oxygen_ppm").cloned().collect());
            o2_perc_chart_data.set(points.iter().filter(|p| p.field == "oxygen_percentage").cloned().collect());
            salinity_chart_data.set(points.iter().filter(|p| p.field == "salinity").cloned().collect());
            conductivity_chart_data.set(points.iter().filter(|p| p.field == "conductivity").cloned().collect());
            ph_chart_data.set(points.iter().filter(|p| p.field == "ph").cloned().collect());
            //chart_data2.set((*points).clone()); // Example: using the same data for now
        }
    });

    // Define series
    // temperature
    
    let temp_series = Series::new(|p: &DataPoint| {
            DateTime::from_timestamp_millis(p.epochtime as i64).unwrap().with_timezone(&FixedOffset::east_opt(3600).unwrap())
    })
    .line(Line::new(|p: &DataPoint| p.value).with_name("Temperature"));

    // pressure
    let press_series = Series::new(|p: &DataPoint| {
        DateTime::from_timestamp_millis(p.epochtime as i64).unwrap().with_timezone(&FixedOffset::east_opt(3600).unwrap())
    })
    .line(Line::new(|p: &DataPoint| p.value).with_name("Pressure")); 

    // oxygen ppm
    let oxppm_series = Series::new(|p: &DataPoint| {
        DateTime::from_timestamp_millis(p.epochtime as i64).unwrap().with_timezone(&FixedOffset::east_opt(3600).unwrap())
    })
    .line(Line::new(|p: &DataPoint| p.value).with_name("Oxygen (ppm)"));

    // oxygen %
    let oxperc_series = Series::new(|p: &DataPoint| {
        DateTime::from_timestamp_millis(p.epochtime as i64).unwrap().with_timezone(&FixedOffset::east_opt(3600).unwrap())
    })
    .line(Line::new(|p: &DataPoint| p.value).with_name("Oxygen (percentage)"));

    // conductivity
    let cond_series = Series::new(|p: &DataPoint| {
        DateTime::from_timestamp_millis(p.epochtime as i64).unwrap().with_timezone(&FixedOffset::east_opt(3600).unwrap())
    })
    .line(Line::new(|p: &DataPoint| p.value).with_name("Conductivity"));

    // salinity
    let sal_series = Series::new(|p: &DataPoint| {
        DateTime::from_timestamp_millis(p.epochtime as i64).unwrap().with_timezone(&FixedOffset::east_opt(3600).unwrap())
    })
    .line(Line::new(|p: &DataPoint| p.value).with_name("Salinity"));

    let ph_series = Series::new(|p: &DataPoint| {
        DateTime::from_timestamp_millis(p.epochtime as i64).unwrap().with_timezone(&FixedOffset::east_opt(3600).unwrap())
    })
    .line(Line::new(|p: &DataPoint| p.value).with_name("pH"));

    view! {
        <h2>"CTD Data"</h2>
        <div class="charts-grid">
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                <SensorChart title="Temperature".to_string() data=temperature_chart_data series=temp_series />
                <SensorChart title="Pressure".to_string() data=pressure_chart_data series=press_series />
                <SensorChart title="Oxygen (ppm)".to_string() data=o2_pmm_chart_data series=oxppm_series.clone() />
                <SensorChart title="Oxygen %".to_string() data=o2_perc_chart_data series=oxperc_series.clone() />
                <SensorChart title="Conductivity".to_string() data=conductivity_chart_data series=cond_series.clone() />
                <SensorChart title="Salinity".to_string() data=salinity_chart_data series=sal_series.clone() />
                <SensorChart title="pH".to_string() data=ph_chart_data series=ph_series.clone() />
            </Suspense>
        </div>
        <Status />
    }
}

// component displaying buttons for asv services
#[component]
fn Status() -> impl IntoView {
    let client = Client::new();
    let client1= Client::new();
    let client2= Client::new();
    let client3= Client::new();

    let result = RwSignal::new(None::<String>);
    let show_popup= RwSignal::new(false);

    view! {
        <h2>"Service Control"</h2>
        <ServiceMonitor />
        <div class="status-container">
            <div class="status-buttons">
                <button class="start" on:click=move |_| spawn_local({
                    let value = client.clone();
                    async move { service_request(value, "IDRONAUT", "start", &result, &show_popup).await}})>"Start CTD gathering"</button>
                <button class="start" on:click=move |_| spawn_local({
                    let value = client2.clone();
                    async move {service_request(value, "camera_capture", "start", &result, &show_popup).await}})>"Start Camera Capture"</button>
                <button class="stop" on:click=move |_| spawn_local({
                    let value = client1.clone();
                    async move {service_request(value, "IDRONAUT", "stop", &result, &show_popup).await}})>"Stop CTD gathering"</button>
                <button class="stop" on:click=move |_| spawn_local({
                    let value = client3.clone();
                    async move {service_request(value, "camera_capture", "stop", &result, &show_popup).await}})>"Stop Camera Capture"</button>
            </div>
            <util::PopUp 
                show_popup=show_popup
                result=result 
            />
        </div>
    }
}

// component displaying service status
#[component]
fn ServiceMonitor() -> impl IntoView {
    // signal
    let tick = RwSignal::new(0);

    // signal interval
    Interval::new(5000, move || {
        tick.update(|t| *t += 1);
    })
    .forget();

    let url = concatcp!(BASEURL, "/api/status");

    // dynamic resource
    let status_resource = LocalResource::new(move || {
        let _ = tick.get();
        let value = url;
        async move {
            let client = Client::new();
            match client.get(value).send().await {
                Ok(response) => match response.json::<ServiceStatus>().await {
                    Ok(status) => {
                        console::log_1(&format!("Fetched status: {:?}", status).into());
                        status
                    }
                    Err(err) => {
                        console::error_1(&format!("Status parse error: {:?}", err).into());
                        ServiceStatus { idronaut: false, camera_capture: false }
                    }
                },
                Err(err) => {
                    console::error_1(&format!("Status request error: {:?}", err).into());
                    ServiceStatus { idronaut: false, camera_capture: false }
                }
            }
        }
    });

    view! {
        <div class="component-container service-monitor">
            <h2>"Service Monitor"</h2>
            <Suspense fallback=move || view! { <div class="status-indicators">
                                <div>
                                    <strong>"CTD gathering: "</strong><p>"Loading..."</p>
                                </div>
                                <div>
                                    <strong>"Camera capture: "</strong><p>"Loading..."</p>
                                </div>
                            </div> }>
                {move || {
                    if let Some(status) = status_resource.get() {
                        view! {
                            <div class="status-indicators">
                                <div>
                                    <strong>"CTD gathering: "</strong>
                                    <p>
                                        { if status.idronaut {
                                            "Running"
                                        } else {
                                            "Stopped"
                                        } }
                                    </p>
                                </div>
                                <div>
                                    <strong>"Camera capture: "</strong>
                                    <p>
                                        { if status.camera_capture {
                                            "Running"
                                        } else {
                                            "Stopped"
                                        } }
                                    </p>
                                </div>
                            </div>
                        }
                    } else {
                        view! { 
                            <div class="status-indicators">
                                <div>
                                    <strong>"CTD gathering: "</strong>
                                    <p>
                                        { "Not available." }
                                    </p>
                                </div>
                                <div>
                                    <strong>"Camera capture: "</strong>
                                    <p>
                                        { "Not available." }
                                    </p>
                                </div>
                            </div>
                        }
                    }
                }}
            </Suspense>
        </div>
    }
}

// web ui header
#[component]
fn SiteHeader() -> impl IntoView {
    view! {
        <header>
            <h2><A href="/">"BRIG_UI"</A></h2>
            <nav>
                <p><A href="/Charts/">"Charts"</A></p>
                <p><A href="/Data/">"Data"</A></p>
                <p><A href="/Status/">"Status"</A></p>
                <p><A href="/Cameras/">"Cameras"</A></p>
            </nav>
        </header>
    }
}