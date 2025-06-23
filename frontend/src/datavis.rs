use web_sys::console;
use leptos_chartistry::*;
use leptos::*;
use reqwest::Client;
use wasm_bindgen::JsCast;
use serde::Deserialize;
use web_sys::{Blob, BlobPropertyBag, Url, HtmlAnchorElement};
use leptos::prelude::*;
use const_format::concatcp;  
use tracing::info;
use chrono::{DateTime, Utc};
use leptos::task::spawn_local;

const BASEURL: &'static str = "http://192.168.2.9:3000";

#[derive(Debug, Clone, serde::Deserialize)]
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

#[derive(Deserialize, Clone, Debug)]
pub struct DataPoint {
    time: String,
    value: f64,
    field: String,
    epochtime: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RTDataPoint {
    pub ph: Option<f64>,
    pub conductivity: Option<f64>,
    pub salinity: Option<f64>,
    pub temperature: Option<f64>,
    pub pressure: Option<f64>,
    pub oxygen_perc: Option<f64>,
    pub oxygen_ppm: Option<f64>,
    pub cog: Option<f64>,
    pub sog: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub depth: Option<f64>,
}

pub fn data_page() -> impl IntoView {
    view! {
        <CsvDownload/>
        <ImageData/>
    }
}

#[component]
pub fn CsvDownload() -> impl IntoView {
    let start_time = RwSignal::new(String::new());
    let end_time = RwSignal::new(String::new());

    let download_csv = move || {
        let start = start_time.get();
        let end = end_time.get();

        spawn_local(async move {
            let url = format!(
                "http://192.168.2.9:3000/api/download_data?start={}&end={}",
                &start,
                &end,
            );

            match reqwest::get(&url).await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(bytes) = resp.bytes().await {
                        let array = js_sys::Uint8Array::from(bytes.as_ref());
                        let blob_parts = js_sys::Array::new();
                        blob_parts.push(&array.buffer());

                        let options = BlobPropertyBag::new();
                        options.set_type("text/csv");

                        let blob = Blob::new_with_buffer_source_sequence_and_options(
                            &blob_parts,
                            &options,
                        )
                        .unwrap();

                        let url = Url::create_object_url_with_blob(&blob).unwrap();
                        let document = web_sys::window().unwrap().document().unwrap();
                        let a = document
                            .create_element("a")
                            .unwrap()
                            .dyn_into::<HtmlAnchorElement>()
                            .unwrap();
                        a.set_href(&url);
                        a.set_download("data.csv");
                        a.click();

                        Url::revoke_object_url(&url).ok();
                    }
                }
                Ok(resp) => {
                    log::error!("Download failed: {}", resp.status());
                }
                Err(err) => {
                    log::error!("Request failed: {:?}", err);
                }
            }
        });
    };

    view! {
        <div class="component-container time-range-downloader">
            <h2>"Download DB data"</h2>
            <details>
                <summary>Instructions</summary>

                <p>"Select start and end date of the range you want the data of."</p>
                <p>"The interface will then download a CSV file containing all the gathered data."</p>
                <p>"This data will be given in rows with the following structure:"</p>
                <p>"result | table | _time | _value | _field | _measurement | camera"</p>
                <p>"The first two columns can be ignored."</p>
                <p>"_time is the timestamp of the acquisition of the entry."</p>
                <p>"_value is the value of the acquisition of the entry."</p>
                <p>"_field is the name of the parameter (eg. latitude, pH, depth, ...)."</p>
                <p>"_measurement is the group of the parameter (GPS, CTD or CAMERA)."</p>
                <p>"camera is a tag used only by camera acquistion entries to denote which of two cameras has made the capture."</p>
            </details>
            <p>"Select range of data to download."</p>
            <div class="form-group">
                <label>
                    "Start:"
                    <input type="datetime-local"
                        on:input=move |e| start_time.set(event_target_value(&e)) />
                </label>
            </div>
            <div class="form-group">
                <label>
                    "End:"
                    <input type="datetime-local"
                        on:input=move |e| end_time.set(event_target_value(&e)) />
                </label>
            </div>
            <div class="form-group">
                <button on:click=move |_| download_csv()>
                    "Download CSV"
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn ImageData() -> impl IntoView {

    let client: RwSignal<Client> = RwSignal::new(Client::new());
    
    // Selection signals for first load
    let selected_camera = RwSignal::new(String::new());
    let selected_date = RwSignal::new(String::new());
    let is_loading_folders = RwSignal::new(false);
    
    // Signals for sets and folders
    let sets = RwSignal::new(Vec::<String>::new());
    let folders_map = RwSignal::new(std::collections::HashMap::<String, Vec<String>>::new());
    
    // Selections and results
    let selected_set = RwSignal::new(String::new());
    let selected_folder = RwSignal::new(String::new());
    let image_num = RwSignal::new(String::from("0001"));
    let image_data = RwSignal::new(None::<ImageDataPoint>);
    let status_message = RwSignal::new(String::new());

    // Fetch sets and folders based on date
    // Fetch sets and folders based on camera and date
    let fetch_folders = move |_| {
        let camera = selected_camera.get();
        let date = selected_date.get();
        
        if camera.is_empty() {
            status_message.set("Please select a camera first.".to_string());
            return;
        }
        
        if date.is_empty() {
            status_message.set("Please select a date first.".to_string());
            return;
        }
        
        status_message.set("Loading folders for selected camera and date...".to_string());
        is_loading_folders.set(true);
        
        // Reset previous selections
        sets.set(Vec::new());
        folders_map.set(std::collections::HashMap::new());
        selected_set.set(String::new());
        selected_folder.set(String::new());
        
        // Get a clone of the client before moving into spawn_local
        let cl = client.get_untracked().clone(); 

        spawn_local(async move {
            let url = format!("http://192.168.2.9:3000/api/camera_folders?camera={}&date={}", camera, date);
            
            match cl.get(&url).send().await {
                Ok(res) => {
                    if res.status().is_success() {
                        match res.json::<Vec<String>>().await {
                            Ok(folder_list) => {
                                // Process folder list into sets and folders
                                let mut map = std::collections::HashMap::new();
                                for entry in folder_list.iter() {
                                    let mut parts = entry.as_str().splitn(2, '/');
                                    if let (Some(set), Some(folder)) = (parts.next(), parts.next()) {
                                        map.entry(set.to_owned())
                                            .or_insert_with(Vec::new)
                                            .push(folder.to_owned());
                                    }
                                }
                                
                                folders_map.set(map.clone());
                                sets.set(map.keys().cloned().collect());
                                status_message.set("Folders loaded successfully.".to_string());
                            },
                            Err(_) => status_message.set("Failed to parse folder data from server.".to_string())
                        }
                    } else {
                        status_message.set(format!("Server error: {}", res.status()));
                    }
                },
                Err(_) => status_message.set("Failed to connect to server.".to_string())
            }
            
            is_loading_folders.set(false);
        });
    };
    // Fetch metadata on button click
    let fetch_metadata = move |_| {
        let camera = selected_camera.get();
        let date = selected_date.get();
        let set = selected_set.get();
        let folder = selected_folder.get();
        let img = image_num.get();

        if camera.is_empty() || date.is_empty() || set.is_empty() || folder.is_empty() || img.is_empty() {
            status_message.set("Missing input, please fill all fields.".to_string());
            return;
        }

        status_message.set("Loading image data...".to_string());
        
        // Get a clone of the client before moving into spawn_local
        let cl = client.get_untracked().clone();
        
        spawn_local(async move {
            let url = format!("http://192.168.2.9:3000/api/image_data?camera={}&date={}&set={}&folder={}&img_num={}", 
                camera, date, set, folder, img);
            
            match cl.get(&url).send().await {
                Ok(res) => {
                    if res.status().is_success() {
                        match res.json::<ImageDataPoint>().await {
                            Ok(data) => {
                                status_message.set("Image data loaded successfully.".to_string());
                                image_data.set(Some(data));
                            },
                            Err(e) => status_message.set("Failed to parse image data from server. ".to_string() + &e.to_string())
                        }
                    } else {
                        status_message.set(format!("Server error: {}", res.status()));
                    }
                },
                Err(_) => status_message.set("Failed to connect to server.".to_string())
            }
        });
    };
    
    view! {
        <div class="component-container camera-browser">
            <h2>"Camera File Browser"</h2>
            <details>
                <summary>Instructions</summary>

                <p>First select the camera, consult camera documentation for the bands of each one.</p>
                <p>Select the date of the day the capture was taken, the system will then give a selection of sets and folders that were used that day.</p>
                <p>The set refers to a startup sequence of the cameras, each time the cameras are powered one, a new set is made.</p>
                <p>Folders contains up to 200 photos, so IMG_0000 to IMG_0199 will be in folder 000, IMG_0200 to IMG_0399 on folder 001 and so on.</p>
                <p>The system will then give all chemical-physical parameters and the GPS coordinates related to the given capture.</p>    
            </details>
            <div class="form-group">
                <label>"Select Camera:"</label>
                <select
                    on:change=move |ev| selected_camera.set(event_target_value(&ev))
                >
                    <option 
                        value="" 
                        selected=move || selected_camera.get().is_empty()
                    >
                        "-- Choose a Camera --"
                    </option>
                    <option 
                        value="cam1" 
                        selected=move || selected_camera.get() == "cam1"
                    >
                        "RedEdge-MX Red"
                    </option>
                    <option 
                        value="cam2" 
                        selected=move || selected_camera.get() == "cam2"
                    >
                        "RedEdge-MX Blue"
                    </option>
                </select>
            </div>

            <div class="form-group">
                <label>"Select Date of Capture:"</label>
                <div class="form-row">
                    <input 
                        type="date" 
                        value=move || selected_date.get() 
                        on:input=move |ev| selected_date.set(event_target_value(&ev)) 
                    />
                    <button 
                        on:click=fetch_folders
                        disabled=move || is_loading_folders.get()
                    >
                        "Load Folders"
                    </button>
                </div>
            </div>

            {move || {
                if is_loading_folders.get() {
                    None
                } else {
                    Some(view! {
                        <div class="form-row">
                            <div class="form-group">
                                <label>"Set:"</label>
                                <select 
                                    on:change=move |ev| {
                                        selected_set.set(event_target_value(&ev));
                                        selected_folder.set("".to_string()); // Reset folder when set changes
                                    }
                                    disabled=move || sets.get().is_empty()
                                >
                                    <option value="">"-- Choose a Set --"</option>
                                    <For
                                        each=move || sets.get().clone()
                                        key=|set| set.clone()
                                        let:set
                                    >
                                    {let value = set.clone(); view! {
                                        <option value={value}>{set}</option>
                                    }}
                                    </For>
                                </select>
                            </div>

                            <div class="form-group">
                                <label>"Folder:"</label>
                                <select 
                                    on:change=move |ev| selected_folder.set(event_target_value(&ev))
                                    disabled=move || selected_set.get().is_empty()
                                >
                                    <option value="">"-- Choose a Folder --"</option>
                                    <For
                                        each=move || {
                                            folders_map
                                                .get()
                                                .get(&selected_set.get())
                                                .cloned()
                                                .unwrap_or_default()
                                        }
                                        key=|folder| folder.clone()
                                        let:folder
                                    >
                                    {let value = folder.clone(); view! {
                                        <option value={value}>{folder}</option>
                                    }}
                                    </For>
                                </select>
                            </div>
                        </div>
                    })
                }
            }}

            <div class="form-group">
                <label>"Image Number (e.g. 0001):"</label>
                <input 
                    type="text" 
                    value=move || image_num.get() 
                    on:input=move |ev| image_num.set(event_target_value(&ev)) 
                    disabled=move || selected_folder.get().is_empty()
                />
            </div>

            <div class="form-group">
                <button 
                    on:click=fetch_metadata
                    disabled=move || {
                        selected_date.get().is_empty() || 
                        selected_set.get().is_empty() || 
                        selected_folder.get().is_empty() || 
                        image_num.get().is_empty()
                    }
                >
                    "Fetch Image Data"
                </button>
                <p class="status-message">{move || status_message.get()}</p>
            </div>

            <div class="form-group">
                <p><strong>"Full Path: "</strong>
                    {move || {
                        if selected_set.get().is_empty() || 
                        selected_folder.get().is_empty() || image_num.get().is_empty() {
                            "".to_string()
                        } else {
                            format!("/files/{}/{}/IMG_{}.tif", 
                                selected_set.get(), 
                                selected_folder.get(), 
                                image_num.get())
                        }
                    }}
                </p>

                {move || image_data.get().map(|data| view! {
                    <div class="image-meta">
                        <p class="timestamp"><strong>"Timestamp: "</strong>{data.date.clone()}</p>
                        
                        <p class="section-header"><strong>GPS data</strong></p>
                        
                        <div class="data-grid">
                            <p><strong>"Latitude: "</strong>{data.lat}"°"</p>
                            <p><strong>"Longitude: "</strong>{data.lon}"°"</p>
                            <p><strong>"Cog: "</strong>{data.cog}"°"</p>
                            <p><strong>"Sog: "</strong>{data.sog}" m/s"</p>
                            <p><strong>"Depth: "</strong>{data.depth}" m"</p>
                        </div>
                        
                        <p class="section-header"><strong>CTD data</strong></p>
                        
                        <div class="data-grid">
                            <p><strong>"Conductivity: "</strong>{data.conductivity}" mS/cm"</p>
                            <p><strong>"Oxygen Percentage: "</strong>{data.oxygen_percentage}</p>
                            <p><strong>"Oxygen PPM: "</strong>{data.oxygen_ppm}" mg/l"</p>
                            <p><strong>"pH: "</strong>{data.ph}</p>
                            <p><strong>"Pressure: "</strong>{data.pressure}" dbar"</p>
                            <p><strong>"Salinity: "</strong>{data.salinity}</p>
                            <p><strong>"Temperature: "</strong>{data.temperature}" °C"</p>
                        </div>
                    </div>
                })}
            </div>
        </div>
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
    res.sort_by_key(|k| k.epochtime);
    //console::log_1(&format!("Ordered vector: {:?}", res).into());
    res
}

// home component, displays charts and service monitoring
#[component]
pub fn Charts() -> impl IntoView {

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
            temperature_chart_data.set(points.iter().filter(|&p| p.field == "temperature").cloned().collect());
            pressure_chart_data.set(points.iter().filter(|&p| p.field == "pressure").cloned().collect());
            o2_pmm_chart_data.set(points.iter().filter(|&p| p.field == "oxygen_ppm").cloned().collect());
            o2_perc_chart_data.set(points.iter().filter(|&p| p.field == "oxygen_percentage").cloned().collect());
            salinity_chart_data.set(points.iter().filter(|&p| p.field == "salinity").cloned().collect());
            conductivity_chart_data.set(points.iter().filter(|&p| p.field == "conductivity").cloned().collect());
            ph_chart_data.set(points.iter().filter(|&p| p.field == "ph").cloned().collect());
            console::log_1(&format!("Ordered vector: {:?}", ph_chart_data.get()).into());
            //chart_data2.set((*points).clone()); // Example: using the same data for now
        }
    });

    // temperature
    let temp_series = Series::new(|p: &DataPoint| {
        p.time.parse::<DateTime<Utc>>().unwrap()})
    .line(Line::new(|p: &DataPoint| p.value).with_interpolation(Interpolation::Linear).with_name("Temperature"));

    // pressure
    let press_series = Series::new(|p: &DataPoint| {
        p.time.parse::<DateTime<Utc>>().unwrap()})
    .line(Line::new(|p: &DataPoint| p.value).with_interpolation(Interpolation::Linear).with_name("Pressure")); 

    // oxygen ppm
    let oxppm_series = Series::new(|p: &DataPoint| {
        p.time.parse::<DateTime<Utc>>().unwrap()})
    .line(Line::new(|p: &DataPoint| p.value).with_interpolation(Interpolation::Linear).with_name("Oxygen (ppm)"));

    // oxygen %
    let oxperc_series = Series::new(|p: &DataPoint| {
        p.time.parse::<DateTime<Utc>>().unwrap()})
    .line(Line::new(|p: &DataPoint| p.value).with_interpolation(Interpolation::Linear).with_name("Oxygen (percentage)"));

    // conductivity
    let cond_series = Series::new(|p: &DataPoint| {
        p.time.parse::<DateTime<Utc>>().unwrap()})
    .line(Line::new(|p: &DataPoint| p.value).with_interpolation(Interpolation::Linear).with_name("Conductivity"));

    // salinity
    let sal_series = Series::new(|p: &DataPoint| {
        p.time.parse::<DateTime<Utc>>().unwrap()})
    .line(Line::new(|p: &DataPoint| p.value).with_interpolation(Interpolation::Linear).with_name("Salinity"));

    let ph_series = Series::new(|p: &DataPoint| {
        p.time.parse::<DateTime<Utc>>().unwrap()})
    .line(Line::new(|p: &DataPoint| p.value).with_interpolation(Interpolation::Linear).with_name("pH"));

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
    }
}

// component displaying chart for given series and data
#[component]
fn SensorChart(title: String, data: RwSignal<Vec<DataPoint>>, series: Series<DataPoint, DateTime<Utc>,  f64>) -> impl IntoView {
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