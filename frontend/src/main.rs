use datavis::RTDataPoint;
use leptos::*;
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::path;
use reqwest::Client;
use gloo::timers::callback::Interval;
use leptos::suspense::Suspense;
use leptos::prelude::*;
use leptos::task::spawn_local;
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
                    Ok(data) => data,
                    Err(_) => RTDataPoint{ph: None, temperature: None, conductivity: None, salinity: None, pressure: None, oxygen_perc: None, oxygen_ppm: None, cog: None, sog: None, latitude: None, longitude: None, depth: None},
                },
                Err(_) => RTDataPoint{ph: None, temperature: None, conductivity: None, salinity: None, pressure: None, oxygen_perc: None, oxygen_ppm: None, cog: None, sog: None, latitude: None, longitude: None, depth: None},
            }
        }
    });
    view!{
        <h2>"Real Time Data"</h2>
        <div class="numbers-grid">
            <Suspense fallback=move || view! {
                <div class="chart-container-small">
                    <div class="number-label">"Latitude"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"Longitude"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"Depth"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"Temperature"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"Pressure"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"Conductivity"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"Salinity"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"pH"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"Oxygen Dissolved %"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"Oxygen Dissolved (ppm)"</div>
                    <div class="number-display">"Loading..."</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"COG"</div>
                    <div class="number-display">"Loading"</div>
                </div>
                <div class="chart-container-small">
                    <div class="number-label">"SOG"</div>
                    <div class="number-display">"Loading"</div>
                </div>
            }>
            {move || {
                if let Some(dt) = data.get() {
                    if let (Some(ph), Some(temperature), Some(pressure), Some(conductivity), Some(salinity), Some(oxygen_perc), Some(oxygen_ppm),
                            Some(cog), Some(sog), Some(latitude), Some(longitude), Some(depth)) = (dt.ph, dt.temperature, dt.pressure, dt.conductivity, dt.salinity, dt.oxygen_perc, dt.oxygen_ppm, dt.cog, dt.sog, dt.latitude, dt.longitude, dt.depth)
                        {
                            view! {
                                <div class="chart-container-small">
                                    <div class="number-label">"Latitude"</div>
                                    <div class="number-display">{latitude.to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Longitude"</div>
                                    <div class="number-display">{longitude.to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Depth"</div>
                                    <div class="number-display">{depth.to_string()+ " m"}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Temperature"</div>
                                    <div class="number-display">{temperature.to_string()+ " °C"}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Pressure"</div>
                                    <div class="number-display">{pressure.to_string() + " dbar"}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Conductivity"</div>
                                    <div class="number-display">{conductivity.to_string()+ " mS/cm"}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Salinity"</div>
                                    <div class="number-display">{salinity.to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"pH"</div>
                                    <div class="number-display">{ph.to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Oxygen Dissolved %"</div>
                                    <div class="number-display">{oxygen_perc.to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Oxygen Dissolved (ppm)"</div>
                                    <div class="number-display">{oxygen_ppm.to_string() + " mg/l"}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"COG"</div>
                                    <div class="number-display">{cog.to_string() + "°"}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"SOG"</div>
                                    <div class="number-display">{sog.to_string() + " m/s"}</div>
                                </div>
                            }
                        } else {
                            view! {
                                <div class="chart-container-small">
                                    <div class="number-label">"Latitude"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Longitude"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Depth"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Temperature"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Pressure"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Conductivity"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Salinity"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"pH"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Oxygen Dissolved %"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"Oxygen Dissolved (ppm)"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"COG"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                                <div class="chart-container-small">
                                    <div class="number-label">"SOG"</div>
                                    <div class="number-display">{"N/A".to_string()}</div>
                                </div>
                            }
                        }
                    } else {
                        view! {
                            <div class="chart-container-small">
                                <div class="number-label">"Latitude"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"Longitude"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"Depth"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"Temperature"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"Pressure"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"Conductivity"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"Salinity"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"pH"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"Oxygen Dissolved %"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"Oxygen Dissolved (ppm)"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"COG"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
                            </div>
                            <div class="chart-container-small">
                                <div class="number-label">"SOG"</div>
                                <div class="number-display">{"N/A".to_string()}</div>
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