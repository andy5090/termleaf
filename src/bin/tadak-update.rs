use std::{
    cmp::Ordering,
    env,
    ffi::OsStr,
    fmt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const REPOSITORY: &str = "https://github.com/andy5090/tadak";
const LATEST_RELEASE_URL: &str = "https://github.com/andy5090/tadak/releases/latest";
const LATEST_INSTALLER_URL: &str =
    "https://github.com/andy5090/tadak/releases/latest/download/tadak-installer.sh";

#[derive(Debug)]
struct UpdateError(String);

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("tadak-update: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), UpdateError> {
    let force = parse_args()?;
    let updater_path = env::current_exe()
        .map_err(|error| UpdateError(format!("cannot locate the updater: {error}")))?;
    let bin_dir = updater_path
        .parent()
        .ok_or_else(|| UpdateError("cannot determine the installation directory".into()))?;
    let tadak_path = bin_dir.join(executable_name("tadak"));
    let installed = match installed_version(&tadak_path) {
        Ok(version) => Some(version),
        Err(_) if force => None,
        Err(error) => return Err(error),
    };

    println!("Checking for Tadak updates...");
    let latest = latest_version()?;

    if !force {
        match installed.as_deref() {
            Some(version) => match compare_versions(version, &latest)? {
                Ordering::Equal => {
                    println!("Tadak {latest} is already up to date.");
                    return Ok(());
                }
                Ordering::Greater => {
                    println!(
                        "Installed Tadak {version} is newer than the latest published release {latest}."
                    );
                    return Ok(());
                }
                Ordering::Less => {}
            },
            None => {
                return Err(UpdateError(
                    "cannot determine the installed Tadak version; use --force to repair it".into(),
                ));
            }
        }
    }

    if force {
        println!("Reinstalling Tadak {latest}...");
    } else {
        println!(
            "Updating Tadak {} -> {latest}...",
            installed.as_deref().unwrap_or("unknown")
        );
    }

    install_latest(bin_dir)?;

    let updated = installed_version(&tadak_path)?;
    if updated != latest {
        return Err(UpdateError(format!(
            "the installer completed, but {tadak_path:?} reports {updated}; expected {latest}"
        )));
    }

    println!("Tadak {updated} installed successfully.");
    Ok(())
}

fn parse_args() -> Result<bool, UpdateError> {
    let mut force = false;
    for argument in env::args().skip(1) {
        match argument.as_str() {
            "--force" => force = true,
            "-h" | "--help" => {
                println!(
                    "Update Tadak to the latest GitHub release.\n\n\
                     Usage: tadak-update [OPTIONS]\n\n\
                     Options:\n  \
                       --force       Reinstall even when the installed version is current\n  \
                     -h, --help     Print help\n  \
                     -V, --version  Print updater version"
                );
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("tadak-update {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            unknown => {
                return Err(UpdateError(format!(
                    "unknown option {unknown:?}; run tadak-update --help"
                )));
            }
        }
    }
    Ok(force)
}

fn installed_version(tadak_path: &Path) -> Result<String, UpdateError> {
    let output = Command::new(tadak_path)
        .arg("--version")
        .output()
        .map_err(|error| {
            UpdateError(format!(
                "cannot run installed Tadak at {tadak_path:?}: {error}; use --force to repair the installation"
            ))
        })?;

    if !output.status.success() {
        return Err(UpdateError(format!(
            "installed Tadak at {tadak_path:?} could not report its version"
        )));
    }

    let output = String::from_utf8(output.stdout)
        .map_err(|_| UpdateError("installed Tadak returned a non-UTF-8 version".into()))?;
    parse_tadak_version(&output)
}

fn latest_version() -> Result<String, UpdateError> {
    let output = Command::new("curl")
        .args([
            "--proto",
            "=https",
            "--tlsv1.2",
            "-LsSf",
            "-o",
            "/dev/null",
            "-w",
            "%{url_effective}",
            LATEST_RELEASE_URL,
        ])
        .output()
        .map_err(|error| UpdateError(format!("cannot run curl: {error}")))?;

    if !output.status.success() {
        return Err(UpdateError(format!(
            "GitHub release check failed with status {}",
            output.status
        )));
    }

    let effective_url = String::from_utf8(output.stdout)
        .map_err(|_| UpdateError("GitHub returned a non-UTF-8 release URL".into()))?;
    parse_release_version(&effective_url)
}

fn install_latest(bin_dir: &Path) -> Result<(), UpdateError> {
    let mut download = Command::new("curl")
        .args([
            "--proto",
            "=https",
            "--tlsv1.2",
            "-LsSf",
            LATEST_INSTALLER_URL,
        ])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| UpdateError(format!("cannot download the installer: {error}")))?;
    let installer_input = download
        .stdout
        .take()
        .ok_or_else(|| UpdateError("cannot read the downloaded installer".into()))?;

    let mut installer = Command::new("sh");
    installer
        .args(["-s", "--", "--no-modify-path"])
        .stdin(Stdio::from(installer_input));

    if let Some(install_root) = cargo_home_for(bin_dir) {
        installer.env("CARGO_HOME", install_root);
    }

    let installer_status = installer
        .status()
        .map_err(|error| UpdateError(format!("cannot run the installer: {error}")))?;
    let download_status = download
        .wait()
        .map_err(|error| UpdateError(format!("cannot finish the download: {error}")))?;

    if !download_status.success() {
        return Err(UpdateError(format!(
            "installer download failed with status {download_status}"
        )));
    }
    if !installer_status.success() {
        return Err(UpdateError(format!(
            "installer failed with status {installer_status}"
        )));
    }

    Ok(())
}

fn cargo_home_for(bin_dir: &Path) -> Option<&Path> {
    (bin_dir.file_name() == Some(OsStr::new("bin")))
        .then(|| bin_dir.parent())
        .flatten()
}

fn executable_name(name: &str) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(format!("{name}.exe"))
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(name)
    }
}

fn parse_tadak_version(output: &str) -> Result<String, UpdateError> {
    let mut fields = output.split_whitespace();
    match (fields.next(), fields.next(), fields.next()) {
        (Some("tadak"), Some(version), None) if !version.is_empty() => Ok(version.to_owned()),
        _ => Err(UpdateError(format!(
            "unexpected Tadak version output: {:?}",
            output.trim()
        ))),
    }
}

fn parse_release_version(url: &str) -> Result<String, UpdateError> {
    let prefix = format!("{REPOSITORY}/releases/tag/v");
    let version = url.trim().strip_prefix(&prefix).ok_or_else(|| {
        UpdateError(format!(
            "GitHub redirected to an unexpected release URL: {:?}",
            url.trim()
        ))
    })?;

    if version.is_empty() || version.contains('/') {
        return Err(UpdateError(format!(
            "GitHub returned an invalid release version: {version:?}"
        )));
    }

    Ok(version.to_owned())
}

fn compare_versions(installed: &str, latest: &str) -> Result<Ordering, UpdateError> {
    Ok(parse_version_numbers(installed)?.cmp(&parse_version_numbers(latest)?))
}

fn parse_version_numbers(version: &str) -> Result<(u64, u64, u64), UpdateError> {
    let core = version.split_once('-').map_or(version, |(core, _)| core);
    let mut numbers = core.split('.');
    let parsed = match (
        numbers.next(),
        numbers.next(),
        numbers.next(),
        numbers.next(),
    ) {
        (Some(major), Some(minor), Some(patch), None) => (
            major.parse::<u64>(),
            minor.parse::<u64>(),
            patch.parse::<u64>(),
        ),
        _ => {
            return Err(UpdateError(format!(
                "unsupported Tadak version format: {version:?}"
            )));
        }
    };

    match parsed {
        (Ok(major), Ok(minor), Ok(patch)) => Ok((major, minor, patch)),
        _ => Err(UpdateError(format!(
            "unsupported Tadak version format: {version:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_installed_version() {
        assert_eq!(parse_tadak_version("tadak 0.1.2\n").unwrap(), "0.1.2");
        assert!(parse_tadak_version("tadak\n").is_err());
        assert!(parse_tadak_version("other 0.1.2\n").is_err());
    }

    #[test]
    fn parses_only_the_expected_release_url() {
        assert_eq!(
            parse_release_version("https://github.com/andy5090/tadak/releases/tag/v0.1.3\n")
                .unwrap(),
            "0.1.3"
        );
        assert!(parse_release_version(
            "https://github.com/someone-else/tadak/releases/tag/v99.0.0"
        )
        .is_err());
    }

    #[test]
    fn derives_cargo_home_from_bin_directory() {
        assert_eq!(
            cargo_home_for(Path::new("/tmp/tadak-home/bin")),
            Some(Path::new("/tmp/tadak-home"))
        );
        assert_eq!(cargo_home_for(Path::new("/tmp/tadak-home")), None);
    }

    #[test]
    fn compares_release_versions_without_allowing_a_downgrade() {
        assert_eq!(compare_versions("0.1.2", "0.1.3").unwrap(), Ordering::Less);
        assert_eq!(compare_versions("0.1.3", "0.1.3").unwrap(), Ordering::Equal);
        assert_eq!(
            compare_versions("0.2.0", "0.1.3").unwrap(),
            Ordering::Greater
        );
        assert!(compare_versions("development", "0.1.3").is_err());
    }
}
