use std::path::Path;
use xshell::{cmd, Shell};

pub fn zip(source: &Path) -> Result<(), xshell::Error> {
    let sh = Shell::new()?;

    if cfg!(windows) {
        cmd!(sh, "powershell Compress-Archive {source} {source}.zip").run()
    } else {
        cmd!(sh, "zip -r {source}.zip {source}").run()
    }
}

pub fn unzip(source: &Path, destination: &Path) -> Result<(), xshell::Error> {
    let sh = Shell::new()?;

    if cfg!(windows) {
        cmd!(sh, "powershell Expand-Archive {source} {destination}").run()
    } else {
        cmd!(sh, "unzip {source} -d {destination}").run()
    }
}

pub fn targz(source: &Path) -> Result<(), xshell::Error> {
    let sh = Shell::new()?;

    let archive_path = format!("{}.tar.gz", source.to_string_lossy());
    let parent_dir = source.parent().unwrap();
    let file_name = source.file_name().unwrap();

    cmd!(sh, "tar -czvf {archive_path} -C {parent_dir} {file_name}").run()
}

pub fn download(url: &str, destination: &Path) -> Result<(), xshell::Error> {
    let sh = Shell::new()?;
    cmd!(sh, "curl -L -o {destination} --url {url}").run()
}

pub fn download_and_extract_zip(url: &str, destination: &Path) -> Result<(), xshell::Error> {
    let sh = Shell::new()?;

    let temp_dir_guard = sh.create_temp_dir()?;

    let zip_file = temp_dir_guard.path().join("temp_download.zip");
    download(url, &zip_file)?;

    sh.remove_path(&destination).ok();
    sh.create_dir(&destination)?;
    unzip(&zip_file, destination)
}

pub fn date_utc_yyyymmdd() -> Result<String, xshell::Error> {
    let sh = Shell::new()?;

    if cfg!(windows) {
        cmd!(
            sh,
            "powershell (Get-Date).ToUniversalTime().ToString(\"yyyy.MM.dd\")"
        )
        .read()
    } else {
        cmd!(sh, "date -u +%Y.%m.%d").read()
    }
}

pub fn copy_recursive(sh: &Shell, source_dir: &Path, dest_dir: &Path) -> Result<(), xshell::Error> {
    sh.create_dir(dest_dir)?;

    for path in sh.read_dir(source_dir)? {
        let dest_path = dest_dir.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_recursive(sh, &path, &dest_path)?;
        } else {
            sh.copy_file(path, dest_path)?;
        }
    }

    Ok(())
}
