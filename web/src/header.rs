use std::collections::HashMap;

use yew::prelude::*;

pub(crate) fn create_header_view(map: &HashMap<Vec<u8>, Vec<u8>>) -> Html {
    map.iter()
        .map(|(key, value)| create_header_field(key, value))
        .collect::<Html>()
}

fn create_header_field(key: &[u8], value: &[u8]) -> Html {
    html! {
        <div class="header-item">
            <span class="header-key">{ String::from_utf8_lossy(key) }</span>
            <span class="header-value">{ String::from_utf8_lossy(value) }</span>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_view_creation() {
        let mut map = HashMap::new();
        map.insert(b"key1".to_vec(), b"value1".to_vec());
        map.insert(b"key2".to_vec(), b"value2".to_vec());
        let actual = create_header_view(&map);
        let expected = html! {
            <>
                <div class="header-item">
                    <span class="header-key">{ String::from("key1") }</span>
                    <span class="header-value">{ String::from("value1") }</span>
                </div>
                <div class="header-item">
                    <span class="header-key">{ String::from("key2") }</span>
                    <span class="header-value">{ String::from("value2") }</span>
                </div>
            </>
        };
        assert_eq!(actual, expected)
    }
}
