use std::ops::Deref;

use drop_area::FileDropArea;
use gloo_file::{futures::read_as_bytes, Blob};
use yew::prelude::*;

mod drop_area;

#[function_component(App)]
fn app() -> Html {
    let dropped_file = use_state(|| None);
    let file_content = use_state(|| None);

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
        let file_content = file_content.clone();
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
                                rrr::JsonDisplay::new(&schema, &body_buf).to_string()
                            });
                            file_content.set(json.ok())
                        }
                    });
                }
            },
            dropped_file,
        );
    }

    let content = if let Some(content) = file_content.as_ref() {
        content.to_string()
    } else {
        String::new()
    };

    html! {
        <>
            <FileDropArea on_drop={on_file_drop} />
            <div>{ file_name }</div>
            <div>{ content }</div>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
