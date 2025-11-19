mod config;
mod handler;
mod html;

use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <>
        <html::Header />
        <html::Body />
        <html::Footer />
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
