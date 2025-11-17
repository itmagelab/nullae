use web_sys::{HtmlInputElement, KeyboardEvent, MouseEvent};
use yew::prelude::*;

use crate::handler;

#[function_component(Header)]
pub(crate) fn header() -> Html {
    html! {}
}

#[function_component(Footer)]
pub(crate) fn footer() -> Html {
    html! {}
}

#[function_component(Body)]
pub fn body() -> Html {
    let input_ref = use_node_ref();
    let response_state = use_state(|| None::<String>);
    let loading_state = use_state(|| false);
    let copy_state = use_state(|| false);

    let on_submit = {
        let input_ref = input_ref.clone();
        let response_state = response_state.clone();
        let loading_state = loading_state.clone();
        let copy_state = copy_state.clone();

        Callback::from(move |_| {
            let input_ref = input_ref.clone();
            let response_state = response_state.clone();
            let loading_state = loading_state.clone();

            if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                let value = input.value();
                if !value.is_empty() {
                    web_sys::console::log_1(&format!("Submitted: {}", value).into());

                    let request = handler::ShortenRequest { url: value.clone() };
                    if !request.validate_url() {
                        let msg = format!("URL validation failed: {}", request.url);
                        response_state.set(Some(msg));
                        copy_state.set(false);
                    } else {
                        let api_url = "http://localhost:3000/api/v1/short".to_string();

                        loading_state.set(true);
                        response_state.set(None);
                        copy_state.set(false);

                        wasm_bindgen_futures::spawn_local(async move {
                            match handler::api_post_shorten(&api_url, request).await {
                                Ok(response) => {
                                    web_sys::console::log_1(
                                        &format!("Request successful: {}", response.short_url)
                                            .into(),
                                    );
                                    response_state.set(Some(response.short_url));
                                }
                                Err(e) => {
                                    web_sys::console::error_1(
                                        &format!("Request failed: {}", e).into(),
                                    );
                                    response_state.set(Some(format!("Error: {}", e)));
                                }
                            }
                            loading_state.set(false);
                        });
                    };

                    input.set_value("");
                }
            }
        })
    };

    let onkeydown = {
        let on_submit = on_submit.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                e.prevent_default();
                on_submit.emit(());
            }
        })
    };

    let on_copy_click = {
        let response_state = response_state.clone();
        let copy_state = copy_state.clone();
        Callback::from(move |_| {
            if let Some(response) = &*response_state {
                let response = response.clone();
                let copy_state = copy_state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(window) = web_sys::window() {
                        let clipboard = window.navigator().clipboard();
                        let promise = clipboard.write_text(&response);
                        if let Ok(_) = wasm_bindgen_futures::JsFuture::from(promise).await {
                            copy_state.set(true);
                            // Reset copy state after 2 seconds
                            let copy_state_clone = copy_state.clone();
                            gloo::timers::callback::Timeout::new(2000, move || {
                                copy_state_clone.set(false);
                            })
                            .forget();
                        }
                    }
                });
            }
        })
    };

    html! {
        <div class="flex flex-col items-center justify-center h-screen bg-gray-100 space-y-4">
            <div class="flex items-center">
                <input
                    ref={input_ref.clone()}
                    type="text"
                    placeholder="Type something..."
                    class="border border-gray-300 rounded-l px-4 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    onkeydown={onkeydown}
                />
                <button
                    onclick={Callback::from(move |_: MouseEvent| on_submit.emit(()))}
                    class="bg-blue-500 text-white px-4 py-2 rounded-r hover:bg-blue-600 transition-colors disabled:bg-blue-300"
                    disabled={*loading_state}
                >
                    { if *loading_state { "Loading..." } else { "Submit" } }
                </button>
            </div>

            if let Some(response) = &*response_state {
                <div class="mt-4 p-4 bg-white rounded-lg shadow-md border border-gray-200 max-w-md w-full">
                    {if response.starts_with("http") {
                        html! {
                            <div class="flex flex-col space-y-2">
                                <a
                                    href={response.clone()}
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="text-blue-600 hover:text-blue-800 underline break-words"
                                >
                                    {response.clone()}
                                </a>
                                <button
                                    onclick={on_copy_click.clone()}
                                    class="text-sm bg-gray-100 hover:bg-gray-200 text-gray-700 px-3 py-1 rounded transition-colors"
                                >
                                    { if *copy_state { "Copied!" } else { "Copy to clipboard" } }
                                </button>
                            </div>
                        }
                    } else {
                        html! {
                            <p class="text-gray-600 break-words">{response}</p>
                        }
                    }}
                </div>
            }
        </div>
    }
}
