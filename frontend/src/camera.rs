use leptos::*;
use reqwest::Client;
use leptos::suspense::Suspense;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use leptos::task::spawn_local;

pub fn CameraPage() -> impl IntoView {
    view! {
        <CameraStatus/>
        <Reformat/>
        <ImageFetch/>
    }
}
#[derive(Serialize, Clone, Deserialize, Debug)]
struct CameraSpace {
    cam1_free: f64,
    cam1_total: f64,
    cam2_free: f64,
    cam2_total: f64,
}

#[component]
pub fn CameraStatus() -> impl IntoView {
    let status_message = RwSignal::new(String::new());
    let camera_data = RwSignal::new(None::<CameraSpace>);
    let fetched = RwSignal::new(false); // for running once
    let client = Client::new();

    Effect::new(move |_| {
        if fetched.get() {
            return;
        }
        fetched.set(true); // run only once

        status_message.set("Loading camera data...".to_string());

        let cl = client.clone();
        spawn_local(async move {
            let url = "http://192.168.2.9:3000/api/camera_status";
            match cl.get(url).send().await {
                Ok(res) => {
                    if res.status().is_success() {
                        match res.json::<CameraSpace>().await {
                            Ok(data) => {
                                status_message.set("Camera data loaded successfully.".to_string());
                                camera_data.set(Some(data));
                            }
                            Err(e) => status_message.set(
                                "Failed to get camera data from server: ".to_string() + &e.to_string(),
                            ),
                        }
                    } else {
                        status_message.set(format!("Server error: {}", res.status()));
                    }
                }
                Err(_) => status_message.set("Failed to connect to server.".to_string()),
            }
        });
    });

    view! {
        <div class="camera_container">
            <p>{move || status_message.get()}</p>
            {
                move || camera_data.get().map(|data| view! {
                    <div class="image-meta">
                        <h2><strong>"Space Available"</strong></h2>
                        <p><strong>"Camera 1: "</strong>{f64::trunc(data.cam1_free  * 100.0) / 100.0} " out of " {f64::trunc(data.cam1_total  * 100.0) / 100.0} " GB"</p>
                        <p><strong>"Camera 2: "</strong>{f64::trunc(data.cam2_free  * 100.0) / 100.0} " out of " {f64::trunc(data.cam2_total  * 100.0) / 100.0} " GB"</p>
                    </div>
                })
            }
        </div>
    }
}

#[component]
pub fn Reformat() -> impl IntoView {
    // Create a signal for storing fetch results
    let result = RwSignal::new(None::<String>);
    
    // Create a signal for controlling popup visibility
    let show_popup= RwSignal::new(false);
    
    // Function to handle the HTTP request
    let fetch_data = move |_| {
        // Define the URL to fetch data from
        let url = "http://192.168.2.9:3000/api/reformat/";
        
        spawn_local(async move {
            // Use fetch API to make a GET request
            match reqwest::get(url).await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text().await {
                            Ok(text) => {
                                // Set the result and show the popup
                                result.set(Some(text));
                                show_popup.set(true);
                            }
                            Err(err) => {
                                result.set(Some(format!("Error parsing response: {}", err)));
                                show_popup.set(true);
                            }
                        }
                    } else {
                        result.set(Some(format!("Error: HTTP status {}", response.status())));
                        show_popup.set(true);
                    }
                }
                Err(err) => {
                    result.set(Some(format!("Request failed: {}", err)));
                    show_popup.set(true);
                }
            }
        });
    };
    
    // Function to close the popup
    let close_popup = move |_| {
        show_popup.set(false);
    };
    
    view! {
        <div class="camera_container">
            <h2>"Reformat Cameras"</h2>
            
            <button 
                on:click=fetch_data
                class="fetch-button"
            >
                "Reformat Camera 1"
            </button>
                        <Show
                when=move || show_popup.get()
                fallback=|| view! { <div></div> }
            >
                <div class="popup-overlay">
                    <div class="popup">
                        <div class="popup-header">
                            <h3>"Result"</h3>
                            <button 
                                on:click=close_popup
                                class="close-button"
                            >
                                "×"
                            </button>
                        </div>
                        <div class="popup-content">
                            {move || result.get().unwrap_or_else(|| "No data".to_string())}
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn ImageFetch() -> impl IntoView {
    let last_capture_image = RwSignal::new(None::<String>);
    let status_message = RwSignal::new(String::new());
    let client: RwSignal<Client> = RwSignal::new(Client::new());
    let fetch_last_capture_image = move |_| {
        status_message.set("Fetching last capture image...".to_string());
        let cl = client.get_untracked().clone();
        spawn_local(async move {
            let url = "http://192.168.2.9:3000/api/get_last_capture?cam=cam1&band=1";
            match cl.get(url).send().await {
                Ok(res) => {
                    if res.status().is_success() {
                        match res.bytes().await {
                            Ok(bytes) => {
                                // Encode the image bytes to base64
                                let base64_str = base64::encode(&bytes);
                                let data_url = format!("data:image/jpeg;base64,{}", base64_str);
                                last_capture_image.set(Some(data_url));
                                status_message.set("Image fetched successfully.".to_string());
                            },
                            Err(_) => status_message.set("Failed to read image bytes.".to_string()),
                        }
                    } else {
                        status_message.set(format!("Server error: {}", res.status()));
                    }
                },
                Err(_) => status_message.set("Failed to connect to server.".to_string())
            }
        });
    };
    view!{
        <div class="camera_container">
            <button on:click=fetch_last_capture_image class="fetch-button">
                "Fetch Last Capture Image"
            </button>
            {move || last_capture_image.get().map(|_data_url| view! {
                <div>
                    <div>
                        <p><strong>"Last Capture Preview:"</strong></p>
                        <p>"Red Band 1 (475±32)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam1&band=1" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Red Band 2 (560±27)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam1&band=2" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Red Band 3 (668±14)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam1&band=3" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Red Band 4 (717±12)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam1&band=4" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Red Band 5 (842±57)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam1&band=5" alt="Last Capture" style="max-width: 500px;" />
                    </div>
                    <div>
                        <p><strong>"Last Capture Preview:"</strong></p>
                        <p>"Blue Band 1 (444±28)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam2&band=1" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Blue Band 2 (560±14)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam2&band=2" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Blue Band 3 (668±16)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam2&band=3" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Blue Band 4 (705±10)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam2&band=4" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Blue Band 5 (740±18)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam2&band=5" alt="Last Capture" style="max-width: 500px;" />
                    </div>
                </div>
            })}
        </div>
    }
}
