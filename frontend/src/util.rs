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

#[component]
pub fn ConfirmationPopup(
    #[prop(into)] show_popup: RwSignal<bool>,
    #[prop(into)] message: Signal<String>,
    #[prop(into)] on_confirm: Callback<()>,
    #[prop(into)] on_cancel: Callback<()>,
) -> impl IntoView {
    view! {
        <Show
            when=move || show_popup.get()
            fallback=|| view! { <div></div> }
        >
            <div class="popup-overlay">
                <div class="popup confirmation-popup">
                    <div class="popup-header">
                        <h3>"Confirmation"</h3>
                    </div>
                    <div class="popup-content">
                        <p>{move || message.get()}</p>
                    </div>
                    <div class="popup-actions">
                        <button
                            on:click=move |_| on_confirm.run(())
                            class="confirm-button"
                        >
                            "Confirm"
                        </button>
                        <button
                            on:click=move |_| on_cancel.run(())
                            class="cancel-button"
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}