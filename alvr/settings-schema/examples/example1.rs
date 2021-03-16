#![allow(dead_code)]

use settings_schema::*;

#[derive(SettingsSchema)]
struct Test1 {
    #[schema(higher_order(
        name = "hello",
        data(boolean(default = false)),
        modifier = r#"{hello1.0.hello2} = {input} * {hello3}"#,
    ))]
    #[schema(advanced)]
    test: bool,

    #[schema(min = 10., gui = "up_down")]
    float: f32,
}

#[derive(SettingsSchema)]
struct Test3 {
    hello: i32,
}

#[derive(SettingsSchema)]
#[schema(gui = "button_group")]
enum Test2 {
    Hello1(#[schema(advanced)] i32),
    Hello2,
    Hello3 {
        hello3_test: bool,
        test1: Test1,
        dict: Vec<(String, Test3)>,
    },
}

fn main() {
    let default = Test2Default {
        variant: Test2DefaultVariant::Hello3,
        Hello1: 3,
        Hello3: Test2Hello3Default {
            hello3_test: true,
            test1: Test1Default {
                test: false,
                float: 3.,
            },
            dict: DictionaryDefault {
                key: "hello".into(),
                value: Test3Default { hello: 0 },
                content: vec![("hello".into(), Test3Default { hello: 1 })],
            },
        },
    };

    println!(
        "default: {}\n",
        serde_json::to_string_pretty(&default).unwrap()
    );

    let schema = Test2::schema(default);

    println!("schema: {}", serde_json::to_string_pretty(&schema).unwrap());
}
