#[cfg(windows)]
fn main() {
    let mut resource = winres::WindowsResource::new();
    resource.set_icon("../dashboard/resources/dashboard.ico");
    resource.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {}
