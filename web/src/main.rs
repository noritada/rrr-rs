use std::ops::Deref;

use drop_area::FileDropArea;
use gloo_file::{futures::read_as_bytes, Blob};
use yew::prelude::*;

mod drop_area;
mod header;
mod tree;

#[function_component(App)]
fn app() -> Html {
    let first_time = use_state(|| true);
    let dropped_file = use_state(|| None);
    let file_content = use_state(|| None);
    let header_fields = use_state(|| None);
    let body_json = use_state(|| None);
    let schema_tree = use_state(|| None);

    let first_time_ = first_time.clone();
    let on_file_drop = {
        let dropped_file = dropped_file.clone();
        Callback::from(move |file: web_sys::File| {
            dropped_file.set(Some(file));
            first_time_.set(false);
        })
    };

    let file_name = if let Some(file) = dropped_file.as_ref() {
        file.name()
    } else {
        String::new()
    };
    let file_size = if let Some(file) = dropped_file.as_ref() {
        format!("{:.0} bytes", file.size())
    } else {
        "--".to_owned()
    };

    let on_drag_over = {
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
            drop_area::display_drop_zone();
        })
    };

    {
        let file_content = file_content.clone();
        let file = dropped_file.clone();
        use_effect_with(dropped_file, move |_| {
            if let Some(file) = file.as_ref() {
                let blob = Blob::from(file.deref().clone());
                wasm_bindgen_futures::spawn_local(async move {
                    let result = read_as_bytes(&blob).await;
                    if let Ok(bytes) = result {
                        let mut reader = rrr::DataReader::new(
                            std::io::Cursor::new(&bytes),
                            rrr::DataReaderOptions::ENABLE_READING_BODY,
                        );
                        let triplet = reader.read();
                        file_content.set(triplet.ok())
                    }
                });
            }
        });
    }

    {
        let header_fields = header_fields.clone();
        let triplet = file_content.clone();
        let file_content = file_content.clone();
        use_effect_with(file_content, move |_| {
            if let Some((_, header, _)) = triplet.as_ref() {
                header_fields.set(Some(header::create_header_view(&header)));
            }
        });
    }

    {
        let schema_tree = schema_tree.clone();
        let triplet = file_content.clone();
        let file_content = file_content.clone();
        use_effect_with(file_content, move |_| {
            if let Some((schema, _, _)) = triplet.as_ref() {
                schema_tree.set(tree::create_schema_tree(&schema.ast).ok());
            }
        });
    }

    {
        let body_json = body_json.clone();
        let triplet = file_content.clone();
        use_effect_with(file_content, move |_| {
            if let Some((schema, _, body_buf)) = triplet.as_ref() {
                let json =
                    rrr::JsonDisplay::new(schema, body_buf, rrr::JsonFormattingStyle::Pretty)
                        .to_string();
                body_json.set(Some(json))
            }
        });
    }

    let file_name = if file_name.is_empty() {
        "--".to_owned()
    } else {
        file_name
    };

    let header_view = if let Some(fields) = header_fields.as_ref() {
        fields.clone()
    } else {
        html! {}
    };

    let schema_tree_view = if let Some(schema_tree) = schema_tree.as_ref() {
        schema_tree.clone()
    } else {
        html! {}
    };

    let body_json = if let Some(json) = body_json.as_ref() {
        json.to_string()
    } else {
        String::new()
    };

    html! {
        <>
            <div id="main" ondragover={ on_drag_over }>
                <div id="menu-pane" class="pane">
                    <h1>{ "Data Viewer" }</h1>
                    <div id="file-info">
                        <div class="file-info-item">
                            <span class="file-info-key">{ "File name" }</span>
                            <span>{ file_name }</span>
                        </div>
                        <div class="file-info-item">
                            <span class="file-info-key">{ "File size" }</span>
                            <span>{ file_size }</span>
                        </div>
                    </div>
                </div>
                <div id="header-pane" class="pane">{ header_view }</div>
                <div id="schema-pane" class="pane tree"><div>{ schema_tree_view }</div></div>
                <div id="view-pane" class="pane">
                    <div>{ body_json }</div>
                </div>
            </div>
            <FileDropArea first_time={*first_time} on_drop={on_file_drop} />
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
