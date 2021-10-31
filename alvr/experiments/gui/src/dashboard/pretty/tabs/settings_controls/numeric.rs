use serde_json as json;
use settings_schema::NumericGuiType;

pub struct IntegerControl {}

impl IntegerControl {
    pub fn new(
        path: String,
        default: i128,
        min: Option<i128>,
        max: Option<i128>,
        step: Option<i128>,
        gui: Option<NumericGuiType>,
        session: json::Value,
    ) -> Self {
        IntegerControl {}
    }
}

pub struct FloatControl {}

impl FloatControl {
    pub fn new(
        path: String,
        default: f64,
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
        gui: Option<NumericGuiType>,
        session: json::Value,
    ) -> Self {
        FloatControl {}
    }
}
