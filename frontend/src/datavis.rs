use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, Url, HtmlAnchorElement};
use leptos::prelude::*;
use leptos::task::spawn_local;


pub fn DataPage() -> impl IntoView {
    let start_time = RwSignal::new(String::new());
    let end_time = RwSignal::new(String::new());

    let download_csv = move || {
        let start = start_time.get();
        let end = end_time.get();

        spawn_local(async move {
            let url = format!(
                "https://your-backend.com/data?start={}&end={}",
                &start,
                &end,
            );

            match reqwest::get(&url).await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(bytes) = resp.bytes().await {
                        let array = js_sys::Uint8Array::from(bytes.as_ref());
                        let blob_parts = js_sys::Array::new();
                        blob_parts.push(&array.buffer());

                        let mut options = BlobPropertyBag::new();
                        options.type_("text/csv");

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
        <p>"Select range of data to download."</p>
        <div class="time-range-downloader">
            <label>
                "Start:"
                <input type="datetime-local"
                    on:input=move |e| start_time.set(event_target_value(&e)) />
            </label>
            <label>
                "End:"
                <input type="datetime-local"
                    on:input=move |e| end_time.set(event_target_value(&e)) />
            </label>
            <button on:click=move |_| download_csv()>
                "Download CSV"
            </button>
        </div>
    }
}