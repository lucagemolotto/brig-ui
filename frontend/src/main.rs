use leptos::*;
use leptos_router::components::{self, Route, Router, Routes, A};
use leptos_router::path;
use reqwest::Client;
use gloo::timers::callback::Interval;
use leptos::suspense::Suspense;
use leptos::prelude::*;
use leptos::task::spawn_local;
use chrono::{DateTime, Utc};
use web_sys::console;
use tracing::info;
use const_format::concatcp;  

mod camera;
mod datavis;
mod util;

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
                    <Route path=path!("/Charts/") view=datavis::Charts/>
                    <Route path=path!("/Status/") view=Status/>
                    <Route path=path!("/Cameras/") view=camera::camera_page/>
                    <Route path=path!("/Data/") view=datavis::data_page/>
                </Routes>
            </main>
        </Router>
    }
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

#[component]
fn Home() -> impl IntoView {
    view!{
        <RTData />
        <Status />
    }

}

#[component]
fn RTData() -> impl IntoView{
    let tick = RwSignal::new(0);

    // signal interval
    Interval::new(5000, move || {
        tick.update(|t| *t += 1);
    })
    .forget();

    let data = LocalResource::new(move || {
        let client = Client::new();
        let _ = tick.get();
        async move { 
            match client.get(concatcp!(BASEURL, "/api/latest_data")).send().await {
                Ok(response) => match response.json::<datavis::RTDataPoint>().await {
                    Ok(data) => Some(data),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        }
    });
    view!{
        <h2>"Real Time Data"</h2>
        <div class="charts-grid">
            <Suspense fallback=move || view! {
                <div class="chart-container">
                <p class="chart-title">pH</p>
                <p>"Loading..."</p>
                </div>
                <div class="chart-container">
                <p class="chart-title">Longitude</p>
                <p>"Loading..."</p>
                </div>
                <div class="chart-container">
                <p class="chart-title">Latitude</p>
                <p>"Loading..."</p>
                </div>
                <div class="chart-container">
                <p class="chart-title">Temperature</p>
                <p>"Loading..."</p>
                </div>
                <div class="chart-container">
                <p class="chart-title">Dissolved Oxygen</p>
                <p>"Loading..."</p>
                </div>
                <div class="chart-container">
                <p class="chart-title">Turbidity</p>
                <p>"Loading..."</p>
                </div>
                <div class="chart-container">
                <p class="chart-title">Conductivity</p>
                <p>"Loading..."</p>
                </div>
                <div class="chart-container">
                <p class="chart-title">Depth</p>
                <p>"Loading..."</p>
                </div>
                <div class="chart-container">
                <p class="chart-title">Flow Rate</p>
                <p>"Loading..."</p>
                </div>
            }>
            {move || {
                let status = data.get();
                match &status {
                    Some(wrapper) => match wrapper.as_ref() {
                        Some(dt) => view! {
                            <div class="chart-container">
                            <p class="chart-title">"Temperature"</p>
                            <p>{dt.temperature.unwrap().to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Pressure"</p>
                            <p>{dt.pressure.unwrap().to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Conductivity"</p>
                            <p>{dt.conductivity.unwrap().to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Salinity"</p>
                            <p>{dt.salinity.unwrap().to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"pH"</p>
                            <p>{dt.ph.unwrap().to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Oxygen Dissolved %"</p>
                            <p>{dt.oxygen_perc.unwrap().to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Oxygen Dissolved (ppm)"</p>
                            <p>{dt.oxygen_ppm.unwrap().to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"COG"</p>
                            <p>{dt.cog.unwrap().to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"SOG"</p>
                            <p>{dt.sog.unwrap().to_string()}</p>
                            </div>
                        },
                        None => view! {
                            <div class="chart-container">
                            <p class="chart-title">"Temperature"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Pressure"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Conductivity"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Salinity"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"pH"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Oxygen Dissolved %"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"Oxygen Dissolved (ppm)"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"COG"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                            <div class="chart-container">
                            <p class="chart-title">"SOG"</p>
                            <p>{"N/A".to_string()}</p>
                            </div>
                        }
                    },
                    None => view! {
                        <div class="chart-container">
                        <p class="chart-title">"Temperature"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                        <div class="chart-container">
                        <p class="chart-title">"Pressure"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                        <div class="chart-container">
                        <p class="chart-title">"Conductivity"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                        <div class="chart-container">
                        <p class="chart-title">"Salinity"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                        <div class="chart-container">
                        <p class="chart-title">"pH"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                        <div class="chart-container">
                        <p class="chart-title">"Oxygen Dissolved %"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                        <div class="chart-container">
                        <p class="chart-title">"Oxygen Dissolved (ppm)"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                        <div class="chart-container">
                        <p class="chart-title">"COG"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                        <div class="chart-container">
                        <p class="chart-title">"SOG"</p>
                        <p>{"N/A".to_string()}</p>
                        </div>
                    }
                }
            }}
            </Suspense>
        </div>
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