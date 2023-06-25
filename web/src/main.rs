use std::ops::Deref;

use drop_area::FileDropArea;
use gloo_file::{futures::read_as_bytes, Blob};
use yew::prelude::*;

mod drop_area;
mod tree;

#[function_component(App)]
fn app() -> Html {
    let dropped_file = use_state(|| None);
    let body_json = use_state(|| None);

    let on_file_drop = {
        let dropped_file = dropped_file.clone();
        Callback::from(move |file: web_sys::File| dropped_file.set(Some(file)))
    };

    let file_name = if let Some(file) = dropped_file.clone().as_ref() {
        file.name()
    } else {
        String::new()
    };

    {
        let body_json = body_json.clone();
        let file = dropped_file.clone();
        use_effect_with_deps(
            move |_| {
                if let Some(file) = file.as_ref() {
                    let blob = Blob::from(file.deref().clone());
                    wasm_bindgen_futures::spawn_local(async move {
                        let result = read_as_bytes(&blob).await;
                        if let Ok(bytes) = result {
                            let mut reader = rrr::DataReader::new(
                                std::io::Cursor::new(&bytes),
                                rrr::DataReaderOptions::ENABLE_READING_BODY,
                            );
                            let json = reader.read().map(|(schema, _, body_buf)| {
                                rrr::JsonDisplay::new(
                                    &schema,
                                    &body_buf,
                                    rrr::JsonFormattingStyle::Pretty,
                                )
                                .to_string()
                            });
                            body_json.set(json.ok())
                        }
                    });
                }
            },
            dropped_file,
        );
    }

    let body_json = if let Some(json) = body_json.as_ref() {
        json.to_string()
    } else {
        String::new()
    };

    html! {
        <>
            <div id="menu-pane" class="pane">
                <div id="menu">
                    <h1>{ "Data Viewer" }</h1>
                    <FileDropArea on_drop={on_file_drop} />
                    <div>{ file_name }</div>
                </div>
            </div>
            <div id="view-pane" class="pane">
                <div>{ body_json }</div>
            </div>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
