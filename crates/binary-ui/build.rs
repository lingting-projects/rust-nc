#[cfg(windows)]
fn win(){
    let mut res = winres::WindowsResource::new();
    res.set_icon("../../icons/256x256.ico");
    res.compile().unwrap();
}

fn main() {
    #[cfg(windows)]
    win()
}