#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("res/launcher.ico");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {}
