use settings_schema::*;

#[allow(dead_code)]
#[derive(SettingsSchema)]
enum ChoiceTest {
    A,

    #[schema(min = -10, max = 10, step = 2, gui = "Slider")]
    B(i32),

    C {
        #[schema(advanced)]
        text_c: String,
    },
}

fn choice_test_default() -> ChoiceTestDefault {
    ChoiceTestDefault {
        variant: ChoiceTestDefaultVariant::B,
        B: 10,
        C: ChoiceTestCDefault {
            text_c: "Hello World".into(),
        },
    }
}

fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&choice_test_schema(choice_test_default())).unwrap()
    );
}
