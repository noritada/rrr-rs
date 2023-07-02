use wasm_bindgen::JsCast;
use web_sys::Element;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct FileDropAreaProps {
    pub on_drop: Callback<web_sys::File>,
}

#[function_component(FileDropArea)]
pub(crate) fn file_drop_area(FileDropAreaProps { on_drop }: &FileDropAreaProps) -> Html {
    let on_drag_over = {
        Callback::from(move |e: DragEvent| {
            e.prevent_default();

            if let Some(target) = e.target() {
                let element = target.unchecked_into::<Element>();
                element
                    .class_list()
                    .add_1("dragover")
                    .expect("adding class 'dragover' failed")
            }
        })
    };
    let on_drag_leave = {
        Callback::from(move |e: DragEvent| {
            e.prevent_default();

            if let Some(target) = e.target() {
                let element = target.unchecked_into::<Element>();
                element
                    .class_list()
                    .remove_1("dragover")
                    .expect("removing class 'dragover' failed")
            }
        })
    };
    let on_file_drop = {
        let on_drop = on_drop.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();

            if let Some(target) = e.target() {
                let element = target.unchecked_into::<Element>();
                element
                    .class_list()
                    .remove_1("dragover")
                    .expect("removing class 'dragover' failed")
            }

            let item = e
                .data_transfer()
                .and_then(|transfer| transfer.files())
                .and_then(|files| files.item(0));
            if let Some(item) = item {
                on_drop.emit(item)
            }
        })
    };

    html! {
        <div id={ "drop-zone" } ondragover={on_drag_over} ondragleave={on_drag_leave} ondrop={on_file_drop}>
            <div id="drop-zone-content">
                { "Drag and drop file here" }
            </div>
        </div>
    }
}
