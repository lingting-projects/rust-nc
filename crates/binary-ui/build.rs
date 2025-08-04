use std::env;

#[cfg(windows)]
fn win(profile: &str) {
    let mut manifest_addition = "".to_string();
    match profile {
        "release" => {
            manifest_addition = r#"
    <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
        <security>
            <requestedPrivileges>
                <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
            </requestedPrivileges>
        </security>
    </trustInfo>
                "#
            .into()
        }
        _ => {}
    }

    let name = "lingting-nc";
    let version = env!("CARGO_PKG_VERSION");
    let manifest = format!(
        r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    <assemblyIdentity
        version="{version}.0"
        name="{name}"
        type="win32"
    />
    {manifest_addition}
</assembly>
    "#
    );
    let mut res = winres::WindowsResource::new();
    res.set_icon("../../icons/256x256.ico")
        .set_manifest(&manifest)
        .set("ProductName", name)
        .set("OriginalFilename", name)
        .set("InternalName", name)
        .set("ApplicationName", name)
        .set("FileDescription", "lingting network control")
        .set("CompanyName", "lingting");
    res.compile().unwrap();
}

fn main() {
    let profile = if let Ok(_p) = env::var("PROFILE") {
        _p
    } else {
        "dev".into()
    };
    #[cfg(windows)]
    win(&profile)
}
