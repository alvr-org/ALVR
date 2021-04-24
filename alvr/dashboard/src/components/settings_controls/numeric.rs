use super::SettingProps;
use settings_schema::SchemaNumeric;
use yew::html;
use yew_functional::function_component;

#[function_component(Integer)]
pub fn integer(props: &SettingProps<SchemaNumeric<i128>, i128>) -> Html {
    html!("integer")
}

#[function_component(Float)]
pub fn float(props: &SettingProps<SchemaNumeric<f64>, f64>) -> Html {
    html!("float")
}
