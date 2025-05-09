use leptos::*;
use leptos::prelude::*;

#[component]
pub fn PopUp(#[prop(into)] show_popup: RwSignal<bool>,
                    #[prop(into)] result: RwSignal<Option<String>>) -> impl IntoView{
    let close_popup = move |_| {
        show_popup.set(false);
    };

    view!{
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
    }
}