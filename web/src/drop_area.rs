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
        })
    };
    let on_file_drop = {
        let on_drop = on_drop.clone();
        Callback::from(move |e: DragEvent| {
            e.prevent_default();
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
        <div ondragover={on_drag_over} ondrop={on_file_drop}>
            { "Please upload a file here" }
        </div>
    }
}
