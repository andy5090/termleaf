//! Private, bounded session storage. Session contents never enter command args.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct SessionStore {
    path: PathBuf,
}

pub(super) struct SessionLock {
    _file: File,
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        // Explicit unlock also releases a Unix lock while a concurrent thread's
        // forked child still holds the inherited descriptor before exec.
        let _ = self._file.unlock();
    }
}

impl SessionStore {
    pub(super) fn new() -> io::Result<Self> {
        let base = if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
                })
        }
        .ok_or_else(unsafe_storage)?;
        if !base.is_absolute() {
            return Err(unsafe_storage());
        }
        Ok(Self {
            path: base.join("termleaf/auth/session.json"),
        })
    }

    fn directory(&self) -> &Path {
        self.path.parent().unwrap()
    }

    pub(super) fn lock(&self) -> io::Result<SessionLock> {
        self.prepare_directory()?;
        let path = self.directory().join(".lock");
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = match options.open(&path) {
            Ok(file) => {
                protect_windows(&path, false)?;
                file
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let meta = fs::symlink_metadata(&path)?;
                if !meta.is_file() || meta.file_type().is_symlink() {
                    return Err(unsafe_storage());
                }
                check_windows(&path)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::{MetadataExt, PermissionsExt};
                    if meta.permissions().mode() & 0o777 != 0o600
                        || meta.nlink() != 1
                        || meta.uid() != fs::metadata(self.directory())?.uid()
                    {
                        return Err(unsafe_storage());
                    }
                }
                OpenOptions::new().read(true).write(true).open(&path)?
            }
            Err(e) => return Err(e),
        };
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => io::Error::new(
                io::ErrorKind::WouldBlock,
                "Another authentication command is running",
            ),
            std::fs::TryLockError::Error(error) => error,
        })?;
        Ok(SessionLock { _file: file })
    }

    fn check_directory(&self) -> io::Result<bool> {
        match fs::symlink_metadata(self.directory()) {
            Ok(meta) => {
                if !meta.is_dir() || meta.file_type().is_symlink() {
                    return Err(unsafe_storage());
                }
                check_windows(self.directory())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if meta.permissions().mode() & 0o777 != 0o700 {
                        return Err(unsafe_storage());
                    }
                }
                Ok(true)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn prepare_directory(&self) -> io::Result<()> {
        if self.check_directory()? {
            return Ok(());
        }
        let directory = self.directory();
        fs::create_dir_all(directory.parent().ok_or_else(unsafe_storage)?)?;
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(directory) {
            Ok(()) => protect_windows(directory, true)?,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e),
        }
        self.check_directory()?;
        Ok(())
    }

    pub(super) fn load(&self) -> io::Result<Option<Vec<u8>>> {
        if !self.check_directory()? {
            return Ok(None);
        }
        let meta = match fs::symlink_metadata(&self.path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 8192 {
            return Err(unsafe_storage());
        }
        check_windows(&self.path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            let directory = fs::metadata(self.directory())?;
            if meta.permissions().mode() & 0o777 != 0o600
                || meta.nlink() != 1
                || meta.uid() != directory.uid()
            {
                return Err(unsafe_storage());
            }
        }
        let mut file = File::open(&self.path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let opened = file.metadata()?;
            if opened.ino() != meta.ino() || opened.dev() != meta.dev() {
                return Err(unsafe_storage());
            }
        }
        let mut bytes = Vec::new();
        (&mut file).take(8193).read_to_end(&mut bytes)?;
        if bytes.len() > 8192 {
            return Err(unsafe_storage());
        }
        Ok(Some(bytes))
    }

    pub(super) fn save(&self, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() > 8192 {
            return Err(unsafe_storage());
        }
        self.prepare_directory()?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = self
            .directory()
            .join(format!(".session-{}-{nonce}", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        let result = (|| {
            protect_windows(&temporary, false)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            // Atomic create without replacing another concurrent login's session.
            // Both names are on the same filesystem, and already private.
            fs::hard_link(&temporary, &self.path)?;
            Ok(())
        })();
        drop(file);
        let removed = fs::remove_file(&temporary);
        result.and(removed)
    }

    pub(super) fn clear(&self) -> io::Result<()> {
        if !self.check_directory()? {
            return Ok(());
        }
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

fn unsafe_storage() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Unsafe cloud session storage",
    )
}

#[cfg(not(windows))]
fn protect_windows(_: &Path, _: bool) -> io::Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn check_windows(_: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn protect_windows(path: &Path, directory: bool) -> io::Result<()> {
    windows_acl(path, if directory { "directory" } else { "file" })
}

#[cfg(windows)]
fn check_windows(path: &Path) -> io::Result<()> {
    windows_acl(path, "check")
}

#[cfg(windows)]
fn windows_acl(path: &Path, action: &str) -> io::Result<()> {
    use std::process::{Command, Stdio};
    // The script is constant; the path is passed as environment data, never
    // interpolated into PowerShell source. A fresh ACL removes explicit grants
    // too, unlike merely removing inherited access with icacls.
    let script = r#"
$ErrorActionPreference = 'Stop'
$path = $env:TERMLeafSessionAclPath
$sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
if ($env:TERMLeafSessionAclAction -eq 'check') {
    $acl = Get-Acl -LiteralPath $path
    if (-not $acl.AreAccessRulesProtected) { exit 1 }
    if ($acl.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { exit 1 }
    foreach ($rule in $acl.GetAccessRules($true, $true, [System.Security.Principal.SecurityIdentifier])) {
        if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Value -ne $sid.Value) { exit 1 }
    }
} else {
    if ($env:TERMLeafSessionAclAction -eq 'directory') {
        $acl = New-Object System.Security.AccessControl.DirectorySecurity
        $rule = New-Object System.Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    } else {
        $acl = New-Object System.Security.AccessControl.FileSecurity
        $rule = New-Object System.Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'Allow')
    }
    $acl.SetOwner($sid)
    $acl.SetAccessRuleProtection($true, $false)
    $acl.AddAccessRule($rule)
    Set-Acl -LiteralPath $path -AclObject $acl
}
"#;
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("TERMLeafSessionAclPath", path)
        .env("TERMLeafSessionAclAction", action)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(unsafe_storage())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                    "termleaf-auth-store-{}-{}-{}",
                    std::process::id(),
                    NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )))
        }
        fn store(&self) -> SessionStore {
            SessionStore {
                path: self.0.join("auth/session.json"),
            }
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn persists_private_session_and_refuses_to_replace_a_concurrent_login() {
        let directory = Directory::new();
        let store = directory.store();
        assert!(store.load().unwrap().is_none());
        store.save(b"first session").unwrap();
        assert_eq!(store.load().unwrap().unwrap(), b"first session");
        assert!(store.save(b"second session").is_err());
        assert_eq!(store.load().unwrap().unwrap(), b"first session");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&store.path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(store.directory())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
        store.clear().unwrap();
        store.clear().unwrap();
        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn rejects_oversized_session() {
        let directory = Directory::new();
        let store = directory.store();
        assert!(store.save(&vec![0; 8193]).is_err());
        store.prepare_directory().unwrap();
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(&store.path)
            .unwrap()
            .write_all(&vec![0; 8193])
            .unwrap();
        assert!(store.load().is_err());
    }

    #[test]
    fn serializes_authentication_commands_and_releases_lock_on_drop() {
        let directory = Directory::new();
        let store = directory.store();
        let lock = store.lock().unwrap();
        assert!(matches!(store.lock(), Err(e) if e.kind()==io::ErrorKind::WouldBlock));
        drop(lock);
        assert!(store.lock().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_or_readable_sessions_and_directories() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let directory = Directory::new();
        let store = directory.store();
        store.prepare_directory().unwrap();
        let target = directory.0.join("untouched");
        fs::write(&target, b"keep me").unwrap();
        symlink(&target, &store.path).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(b"secret").is_err());
        assert_eq!(fs::read(&target).unwrap(), b"keep me");
        store.clear().unwrap();
        store.save(b"secret").unwrap();
        fs::set_permissions(&store.path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(store.load().is_err());
        fs::set_permissions(store.directory(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(store.load().is_err());
        assert!(store.clear().is_err());
        fs::set_permissions(store.directory(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::remove_dir_all(store.directory()).unwrap();
        symlink(&directory.0, store.directory()).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(b"secret").is_err());
    }
}
