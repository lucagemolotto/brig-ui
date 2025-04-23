use leptos::*;
use reqwest::Client;
use leptos::suspense::Suspense;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn CameraPage() -> impl IntoView {

    let client = RwSignal::new(Client::new());

    // Fetch all folders once
    let folders_resource = LocalResource::new(
        move || {
            let client = client.get_untracked().clone();
            async move {
                client
                    .get("/api/folders")
                    .send()
                    .await
                    .ok()?
                    .json::<String>()
                    .await
                    .ok()
            }
        }
    );

    // Signals
    let sets = RwSignal::new(Vec::<String>::new());
    let folders_map = RwSignal::new(std::collections::HashMap::<String, Vec<String>>::new());

    let selected_set = RwSignal::new(String::new());
    let selected_folder = RwSignal::new(String::new());
    let image_num = RwSignal::new(String::from("0001"));
    let image_data = RwSignal::new(None::<ImageData>);
    let status_message = RwSignal::new(String::new());

    // Extract sets & folders from raw list
    Effect::new(move |_| {
        if let Some(folder_list) = folders_resource.get() {
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
        }
    });

    // Fetch metadata on button click - FIXED VERSION
    let fetch_metadata = move |_| {
        let set = selected_set.get();
        let folder = selected_folder.get();
        let img = image_num.get();

        if set.is_empty() || folder.is_empty() || img.is_empty() {
            status_message.set("Missing input, please fill all fields.".to_string());
            return;
        }

        status_message.set("Loading data...".to_string());
        
        // Get a clone of the client before moving into spawn_local
        let cl = client.get_untracked().clone();
        
        spawn_local(async move {
            let url = format!("/api/image-data?set={}&folder={}&img_num={}", set, folder, img);
            
            match cl.get(&url).send().await {
                Ok(res) => {
                    if res.status().is_success() {
                        match res.json::<ImageData>().await {
                            Ok(data) => {
                                status_message.set("Data loaded successfully.".to_string());
                                image_data.set(Some(data));
                            },
                            Err(_) => status_message.set("Failed to parse data from server.".to_string())
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
        <h2>"Camera File Browser"</h2>

        <Suspense fallback=move || view! { <p>"Loading folders..."</p> }>
            {move || folders_resource.get().map(|_| view! {
                <div>
                    <label>"Set:"</label>
                    <select on:change=move |ev| {
                        selected_set.set(event_target_value(&ev));
                        selected_folder.set("".to_string()); // Reset folder when set changes
                    }>
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

                <div>
                    <label>"Folder:"</label>
                    <select on:change=move |ev| selected_folder.set(event_target_value(&ev))>
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
            })}
        </Suspense>

        <div>
            <label>"Image Number (e.g. 0001):"</label>
            <input 
                type="number" 
                min="0" max="9999"
                value=move || image_num.get() 
                on:input=move |ev| image_num.set(event_target_value(&ev)) 
            />
        </div>

        <div class="mt-2">
            <button on:click=fetch_metadata>"Fetch Image Data"</button>
            <p class="status-message">{move || status_message.get()}</p>
        </div>

        <div class="mt-4">
            <p><strong>"Full Path: "</strong>
                {move || {
                    if selected_set.get().is_empty() || selected_folder.get().is_empty() || image_num.get().is_empty() {
                        "".to_string()
                    } else {
                        format!("/files/{}/{}/IMG_{}.tif", selected_set.get(), selected_folder.get(), image_num.get())
                    }
                }}
            </p>

            {move || image_data.get().map(|data| view! {
                <div class="image-meta">
                    <p><strong>"Timestamp: "</strong>{data.timestamp.clone()}</p>
                    <p><strong>"pH: "</strong>{data.ph}</p>
                </div>
            })}
        </div>
    }
}

// Example struct returned by your backend
#[derive(Debug, Clone, serde::Deserialize)]
struct ImageData {
    timestamp: String,
    ph: f32,
    // Add other fields as needed
}