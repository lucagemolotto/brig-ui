use base64::{Engine as _, engine::general_purpose};
use leptos::*;
use reqwest::Client;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use leptos::task::spawn_local;

/*
Camera utilities and info visualization
 */

pub fn camera_page() -> impl IntoView {
    view! {
        <CameraStatus/> // Status of the cameras (space available)
        <Reformat/> // Window for reformatting SD cards
        <ImageFetch/> // Fetcher of latest capture
    }
}
#[derive(Serialize, Clone, Deserialize, Debug)]
struct CameraSpace {
    cam1_free: f64,
    cam1_total: f64,
    cam2_free: f64,
    cam2_total: f64,
}


/* 
This component handles the status of the micasense cameras,
it simply uses the RedEdge APIs to fetch the total and the free space and shows it.
*/
#[component]
pub fn CameraStatus() -> impl IntoView {
    let status_message = RwSignal::new(String::new());
    let camera_data = RwSignal::new(None::<CameraSpace>);
    let fetched = RwSignal::new(false); // for running once
    let client = Client::new();

    // Effect for the asynchronous HTTP API call
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
        <div class="camera-container">
            <h2>"Camera Info"</h2>
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

/* 
This component handles the reformatting of the micasense cameras,
using the RedEdge HTTP APIs.
*/
#[component]
pub fn Reformat() -> impl IntoView {    
    // Create a signal for storing fetch results
    let result = RwSignal::new(None::<String>);
    
    // Create signals for controlling popups visibility
    let show_result_popup = RwSignal::new(false);
    let show_confirmation_popup = RwSignal::new(false);
    
    // Create a signal to store which camera is being reformatted
    let target_camera = RwSignal::new(String::new());
    
    // Function to handle the HTTP request
    let fetch_data = move |camera: &str| {
        // Define the URL to fetch data from with camera parameter
        let url = format!("http://192.168.2.9:3000/api/reformat?camera={}", camera);
        
        spawn_local(async move {
            // Use fetch API to make a GET request
            match reqwest::get(url).await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text().await {
                            Ok(text) => {
                                // Set the result and show the result popup
                                result.set(Some(text));
                                show_result_popup.set(true);
                            }
                            Err(err) => {
                                result.set(Some(format!("Error parsing response: {}", err)));
                                show_result_popup.set(true);
                            }
                        }
                    } else {
                        result.set(Some(format!("Error: HTTP status {}", response.status())));
                        show_result_popup.set(true);
                    }
                }
                Err(err) => {
                    result.set(Some(format!("Request failed: {}", err)));
                    show_result_popup.set(true);
                }
            }
        });
    };
    
    // Function to show confirmation popup
    let request_confirmation = move |camera: &str| {
        target_camera.set(camera.to_string());
        show_confirmation_popup.set(true);
    };
    
    // Function to handle confirmation
    let on_confirm = move |_| {
        let camera = target_camera.get();
        show_confirmation_popup.set(false);
        fetch_data(&camera);
    };
    
    // Function to handle cancellation
    let on_cancel = move |_| {
        show_confirmation_popup.set(false);
    };
    
    view! {
        <div class="camera-container">
            <h2>"Reformat Cameras"</h2>
            <div class="button-container">
                <button
                    on:click=move |_| request_confirmation("cam1")
                    class="fetch-button"
                >
                    "Reformat Camera 1"
                </button>
                <button
                    on:click=move |_| request_confirmation("cam2")
                    class="fetch-button"
                >
                    "Reformat Camera 2"
                </button>
            </div>
            
            // Confirmation popup
            <crate::util::ConfirmationPopup
                show_popup=show_confirmation_popup
                message=Signal::derive(move || format!("Are you sure you want to reformat {}? This action cannot be undone.", 
                    if target_camera.get() == "cam1" { "Camera 1" } else { "Camera 2" }))
                on_confirm=Callback::new(on_confirm)
                on_cancel=Callback::new(on_cancel)
            />
            
            // Result popup
            <crate::util::PopUp
                show_popup=show_result_popup
                result=result
            />
        </div>
    }
}

/* 
This component fetches the images of the latest capture.
It does this by simply calling get_last_capture on the backend, which handles basically everything.
*/
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
                                let base64_str = general_purpose::STANDARD.encode(&bytes);
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
        <div class="camera-container">
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
                        <p>"Blue Band 2 (531±14)" </p>
                        <img src="http://192.168.2.9:3000/api/get_last_capture?cam=cam2&band=2" alt="Last Capture" style="max-width: 500px;" />
                        <p>"Blue Band 3 (650±16)" </p>
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
