use std::{
    cmp::Ordering,
    env,
    ffi::OsStr,
    fmt,
    path::Path,
    process::{Command, Stdio},
};

const REPOSITORY: &str = "https://github.com/andy5090/termleaf";
const LATEST_RELEASE_URL: &str = "https://github.com/andy5090/termleaf/releases/latest";
const LATEST_INSTALLER_URL: &str =
    "https://github.com/andy5090/termleaf/releases/latest/download/termleaf-installer.sh";

#[derive(Debug)]
pub(crate) struct UpdateError(String);

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub(crate) fn run(force: bool) -> Result<(), UpdateError> {
    let termleaf_path = env::current_exe()
        .map_err(|error| UpdateError(format!("cannot locate Termleaf: {error}")))?;
    let bin_dir = termleaf_path
        .parent()
        .ok_or_else(|| UpdateError("cannot determine the installation directory".into()))?;
    let installed = match installed_version(&termleaf_path) {
        Ok(version) => Some(version),
        Err(_) if force => None,
        Err(error) => return Err(error),
    };

    println!("Checking for Termleaf updates...");
    let latest = latest_version()?;

    if !force {
        match installed.as_deref() {
            Some(version) => match compare_versions(version, &latest)? {
                Ordering::Equal => {
                    println!("Termleaf {latest} is already up to date.");
                    return Ok(());
                }
                Ordering::Greater => {
                    println!(
                        "Installed Termleaf {version} is newer than the latest published release {latest}."
                    );
                    return Ok(());
                }
                Ordering::Less => {}
            },
            None => {
                return Err(UpdateError(
                    "cannot determine the installed Termleaf version; use --force to repair it"
                        .into(),
                ));
            }
        }
    }

    if force {
        println!("Reinstalling Termleaf {latest}...");
    } else {
        println!(
            "Updating Termleaf {} -> {latest}...",
            installed.as_deref().unwrap_or("unknown")
        );
    }

    install_latest(bin_dir)?;

    let updated = installed_version(&termleaf_path)?;
    if updated != latest {
        return Err(UpdateError(format!(
            "the installer completed, but {termleaf_path:?} reports {updated}; expected {latest}"
        )));
    }

    println!("Termleaf {updated} installed successfully.");
    Ok(())
}

fn installed_version(termleaf_path: &Path) -> Result<String, UpdateError> {
    let output = Command::new(termleaf_path)
        .arg("--version")
        .output()
        .map_err(|error| {
            UpdateError(format!(
                "cannot run installed Termleaf at {termleaf_path:?}: {error}; use --force to repair the installation"
            ))
        })?;

    if !output.status.success() {
        return Err(UpdateError(format!(
            "installed Termleaf at {termleaf_path:?} could not report its version"
        )));
    }

    let output = String::from_utf8(output.stdout)
        .map_err(|_| UpdateError("installed Termleaf returned a non-UTF-8 version".into()))?;
    parse_termleaf_version(&output)
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
        return Err(curl_error(
            "GitHub release check failed",
            output.status.code(),
            &output.stderr,
        ));
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
        .stderr(Stdio::piped())
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
    let download_output = download
        .wait_with_output()
        .map_err(|error| UpdateError(format!("cannot finish the download: {error}")))?;

    if !download_output.status.success() {
        return Err(curl_error(
            "Installer download failed",
            download_output.status.code(),
            &download_output.stderr,
        ));
    }
    if !installer_status.success() {
        return Err(UpdateError(format!(
            "installer failed with status {installer_status}"
        )));
    }

    Ok(())
}

fn curl_error(context: &str, code: Option<i32>, stderr: &[u8]) -> UpdateError {
    let reason = match code {
        Some(5) => "the configured proxy address could not be resolved. Check your proxy and DNS settings",
        Some(6) => "github.com could not be resolved. Check your internet connection and DNS settings",
        Some(7) => "GitHub could not be reached. Check your internet connection, firewall, or proxy",
        Some(22) => "GitHub returned an HTTP error. The service or release may be temporarily unavailable",
        Some(28) => "the connection timed out. Check your internet connection and try again",
        Some(35) => "the TLS/SSL connection failed. Check your system clock and TLS configuration",
        Some(60) => "GitHub's TLS certificate could not be verified. Check your system clock and CA certificates",
        Some(_) => "curl could not complete the request. Check the diagnostic below",
        None => "curl was interrupted before it could complete the request",
    };
    let status = code.map_or_else(|| "unknown".to_owned(), |code| code.to_string());
    let diagnostic = String::from_utf8_lossy(stderr);
    let diagnostic = diagnostic.trim();

    if diagnostic.is_empty() {
        UpdateError(format!("{context}: {reason} (curl exit code {status})"))
    } else {
        UpdateError(format!(
            "{context}: {reason} (curl exit code {status})\nCurl diagnostic: {diagnostic}"
        ))
    }
}

fn cargo_home_for(bin_dir: &Path) -> Option<&Path> {
    (bin_dir.file_name() == Some(OsStr::new("bin")))
        .then(|| bin_dir.parent())
        .flatten()
}

fn parse_termleaf_version(output: &str) -> Result<String, UpdateError> {
    let mut fields = output.split_whitespace();
    match (fields.next(), fields.next(), fields.next()) {
        (Some("termleaf"), Some(version), None) if !version.is_empty() => Ok(version.to_owned()),
        _ => Err(UpdateError(format!(
            "unexpected Termleaf version output: {:?}",
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
                "unsupported Termleaf version format: {version:?}"
            )));
        }
    };

    match parsed {
        (Ok(major), Ok(minor), Ok(patch)) => Ok((major, minor, patch)),
        _ => Err(UpdateError(format!(
            "unsupported Termleaf version format: {version:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_installed_version() {
        assert_eq!(parse_termleaf_version("termleaf 0.2.0\n").unwrap(), "0.2.0");
        assert!(parse_termleaf_version("termleaf\n").is_err());
        assert!(parse_termleaf_version("other 0.2.0\n").is_err());
    }

    #[test]
    fn parses_only_the_expected_release_url() {
        assert_eq!(
            parse_release_version("https://github.com/andy5090/termleaf/releases/tag/v0.2.0\n")
                .unwrap(),
            "0.2.0"
        );
        assert!(parse_release_version(
            "https://github.com/someone-else/termleaf/releases/tag/v99.0.0"
        )
        .is_err());
    }

    #[test]
    fn derives_cargo_home_from_bin_directory() {
        assert_eq!(
            cargo_home_for(Path::new("/tmp/termleaf-home/bin")),
            Some(Path::new("/tmp/termleaf-home"))
        );
        assert_eq!(cargo_home_for(Path::new("/tmp/termleaf-home")), None);
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

    #[test]
    fn explains_dns_failures_with_an_actionable_message() {
        let error = curl_error(
            "GitHub release check failed",
            Some(6),
            b"curl: (6) Could not resolve host: github.com\n",
        );

        assert!(error.0.contains("github.com could not be resolved"));
        assert!(error.0.contains("internet connection and DNS settings"));
        assert!(error.0.contains("curl exit code 6"));
        assert!(error.0.contains("Could not resolve host"));
    }

    #[test]
    fn explains_other_common_curl_failures() {
        for (code, expected) in [
            (5, "proxy address"),
            (7, "firewall"),
            (22, "HTTP error"),
            (28, "timed out"),
            (35, "TLS/SSL connection"),
            (60, "TLS certificate"),
        ] {
            assert!(curl_error("Update failed", Some(code), b"")
                .0
                .contains(expected));
        }
    }
}
