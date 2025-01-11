use winresource::VersionInfo;

fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon_with_id("resources/icon.ico", "32152");
    res.set("OriginalFilename", "tacky-borders.exe");
    res.set("ProductName", "TackyBorders");
    res.set("FileDescription", "TackyBorders");
    res.set("LegalCopyRight", "Copyright © 2024 0xJWLabs");

    let version_parts = env!("CARGO_PKG_VERSION")
        .split('.')
        .take(3)
        .map(|part| part.parse().expect("Failed to parse version number"))
        .collect::<Vec<u16>>();

    let [major, minor, patch] = <[u16; 3]>::try_from(version_parts)
        .expect("CARGO_PKG_VERSION must have exactly three components (major.minor.patch)");

    let build_number = std::env::var("BUILD_NUMBER")
        .map(|v| v.parse::<u16>().unwrap_or(0))
        .unwrap_or(0); // Default to 0 if the variable is missing

    let version_str = format!("{}.{}.{}.{}", major, minor, patch, build_number);
    res.set("FileVersion", &version_str);
    res.set("ProductVersion", &version_str);

    let version_u64 = ((major as u64) << 48) | ((minor as u64) << 32) | ((patch as u64) << 16);

    res.set_version_info(VersionInfo::FILEVERSION, version_u64);
    res.set_version_info(VersionInfo::PRODUCTVERSION, version_u64);

    res.compile().unwrap();
}
