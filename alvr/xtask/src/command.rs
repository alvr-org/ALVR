use std::path::Path;
use xshell::{cmd, Shell};

pub fn zip(sh: &Shell, source: &Path) -> Result<(), xshell::Error> {
    let _push_guard = sh.push_dir(source);
    cmd!(sh, "zip -r9X {source} .").run()
}

pub fn unzip(sh: &Shell, source: &Path, destination: &Path) -> Result<(), xshell::Error> {
    cmd!(sh, "unzip {source} -d {destination}").run()
}

pub fn untar(sh: &Shell, source: &Path, destination: &Path) -> Result<(), xshell::Error> {
    cmd!(sh, "tar -xvf {source} -C {destination}").run()
}

pub fn targz(sh: &Shell, source: &Path) -> Result<(), xshell::Error> {
    let parent_dir = source.parent().unwrap();
    let file_name = source.file_name().unwrap();

    cmd!(sh, "tar -czvf {source}.tar.gz -C {parent_dir} {file_name}").run()
}

pub fn download(sh: &Shell, url: &str, destination: &Path) -> Result<(), xshell::Error> {
    cmd!(sh, "curl -L -o {destination} --url {url}").run()
}

pub fn download_and_extract_zip(url: &str, destination: &Path) -> Result<(), xshell::Error> {
    let sh = Shell::new().unwrap();
    let temp_dir_guard = sh.create_temp_dir()?;

    let zip_file = temp_dir_guard.path().join("temp_download.zip");
    download(&sh, url, &zip_file)?;

    unzip(&sh, &zip_file, destination)
}

pub fn download_and_extract_tar(url: &str, destination: &Path) -> Result<(), xshell::Error> {
    let sh = Shell::new().unwrap();
    let temp_dir_guard = sh.create_temp_dir()?;

    let tar_file = temp_dir_guard.path().join("temp_download.tar");
    download(&sh, url, &tar_file)?;

    untar(&sh, &tar_file, destination)
}

pub fn date_utc_yyyymmdd(sh: &Shell) -> Result<String, xshell::Error> {
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
