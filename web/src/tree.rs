use rrr::{Ast, AstKind, AstVisitor, Error, Len};
use yew::prelude::*;

pub(crate) fn create_schema_tree(ast: &Ast) -> Result<Html, Error> {
    let mut formatter = SchemaTreeFormatter;
    formatter.visit(ast)
}

struct SchemaTreeFormatter;

impl AstVisitor for SchemaTreeFormatter {
    type ResultItem = Html;

    fn visit_struct(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        if let Ast {
            kind: AstKind::Struct(children),
            ..
        } = node
        {
            let children_html = children
                .iter()
                .filter_map(|child| self.visit(child).ok())
                .map(|c| html! { <li>{ c }</li> })
                .collect::<Html>();

            let html = html! {
                <>
                    { create_node(node) }
                    <ul>{ children_html }</ul>
                </>
            };
            Ok(html)
        } else {
            unreachable!()
        }
    }

    fn visit_array(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        if let Ast {
            kind: AstKind::Array(_, child),
            ..
        } = node
        {
            let html = html! {
                <>
                    { create_node(node) }
                    <ul>
                        <li>{ self.visit(child)? }</li>
                    </ul>
                </>
            };
            Ok(html)
        } else {
            unreachable!()
        }
    }

    fn visit_builtin(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        let html = create_node(node);
        Ok(html)
    }
}

fn create_node(node: &Ast) -> Html {
    let name = prettify_special_field_name(&node.name);
    htmlify(name, &node.kind)
}

fn htmlify(name: &str, kind: &AstKind) -> Html {
    let kind = match kind {
        AstKind::Int8 => "INT8".to_owned(),
        AstKind::Int16 => "INT16".to_owned(),
        AstKind::Int32 => "INT32".to_owned(),
        AstKind::UInt8 => "UINT8".to_owned(),
        AstKind::UInt16 => "UINT16".to_owned(),
        AstKind::UInt32 => "UINT32".to_owned(),
        AstKind::Float32 => "FLOAT32".to_owned(),
        AstKind::Float64 => "FLOAT64".to_owned(),
        AstKind::Str => "STR".to_owned(),
        AstKind::NStr(n) => format!("<{n}>NSTR"),
        AstKind::Struct(..) => "Struct".to_owned(),
        AstKind::Array(len, ..) => {
            let len = match len {
                Len::Fixed(n) => format!("fixed ({n})"),
                Len::Variable(s) => format!("variable ({s})"),
                Len::Unlimited => "unlimited".to_owned(),
            };
            format!("Array (length: {len})")
        }
    };
    html! {
        <><span class="name">{ name }</span><span class="type">{ kind }</span></>
    }
}

fn prettify_special_field_name(name: &str) -> &str {
    match name {
        "" => "/",
        "[]" => "[index]",
        s => s,
    }
}

#[cfg(test)]
mod tests {
    use rrr::Schema;

    use super::*;

    macro_rules! test_schema_tree_display {
        ($(($name:ident, $input:expr, $expected:expr),)*) => ($(
            #[test]
            fn $name() {
                let input = $input;
                let schema = input.parse::<Schema>().unwrap();
                let actual = create_schema_tree(&schema.ast).unwrap();
                let expected = $expected;

                assert_eq!(actual, expected);
            }
        )*);
    }

    test_schema_tree_display! {
        (
            schema_tree_display_for_data_with_fixed_length_builtin_type_array,
            "fld1:{3}INT8",
            html! {
                <>
                    <>
                        <span class="name">{ "/" }</span>
                        <span class="type">{ "Struct" }</span>
                    </>
                    <ul>
                        <>
                            <li>
                                <>
                                    <>
                                        <span class="name">{ "fld1" }</span>
                                        <span class="type">{ "Array (length: fixed (3))" }</span>
                                    </>
                                    <ul>
                                        <li>
                                            <>
                                                <span class="name">{ "[index]" }</span>
                                                <span class="type">{ "INT8" }</span>
                                            </>
                                        </li>
                                    </ul>
                                </>
                            </li>
                        </>
                    </ul>
                </>
            }
        ),
    }
}
