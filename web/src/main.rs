use yew::prelude::*;
mod drop_area;
use drop_area::FileDropArea;

#[function_component(App)]
fn app() -> Html {
    let dropped_file = use_state(|| None);
    let on_file_drop = {
        let dropped_file = dropped_file.clone();
        Callback::from(move |file: web_sys::File| dropped_file.set(Some(file)))
    };

    let file_name = if let Some(file) = dropped_file.as_ref() {
        file.name()
    } else {
        String::new()
    };

    html! {
        <>
            <FileDropArea on_drop={on_file_drop} />
            <div>{ file_name }</div>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
