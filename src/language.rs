//! Optional, data-only language packs and their enlarged-text glyphs.

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const PACK_MAGIC: &[u8; 5] = b"TLGP1";
const GLYPH_ROWS: usize = 10;
const GLYPH_RECORD_SIZE: usize = 4 + 1 + GLYPH_ROWS * 2;
const RELEASE_BASE: &str = "https://github.com/andy5090/termleaf/releases/download";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    English,
    Korean,
    Japanese,
}

impl Language {
    pub const ALL: [Self; 3] = [Self::English, Self::Korean, Self::Japanese];

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::English),
            "ko" => Some(Self::Korean),
            "ja" => Some(Self::Japanese),
            _ => None,
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Korean => "ko",
            Self::Japanese => "ja",
        }
    }

    pub const fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Korean => "한국어",
            Self::Japanese => "日本語",
        }
    }

    pub const fn is_builtin(self) -> bool {
        matches!(self, Self::English)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedGlyph {
    pub width: usize,
    pub rows: [u16; GLYPH_ROWS],
}

#[derive(Debug, Clone)]
struct LanguagePack {
    glyphs: BTreeMap<char, PackedGlyph>,
}

#[derive(Debug, Clone)]
pub struct LanguageRegistry {
    data_dir: PathBuf,
    packs: BTreeMap<Language, LanguagePack>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageError(String);

impl fmt::Display for LanguageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl LanguageRegistry {
    pub fn load() -> Self {
        Self::load_from(data_dir())
    }

    pub fn load_from(data_dir: PathBuf) -> Self {
        let mut registry = Self {
            data_dir,
            packs: BTreeMap::new(),
        };
        for language in [Language::Korean, Language::Japanese] {
            if let Ok(pack) = load_pack(&registry.pack_dir(language), language) {
                registry.packs.insert(language, pack);
            }
        }
        registry
    }

    pub fn is_installed(&self, language: Language) -> bool {
        language.is_builtin() || self.packs.contains_key(&language)
    }

    pub fn supports_live_input(&self, language: Language) -> bool {
        match language {
            Language::English => false,
            Language::Korean => self.is_installed(language),
            Language::Japanese => {
                self.is_installed(language)
                    && [
                        "unigram.model",
                        "bigram.model",
                        "skip_bigram.model",
                        "SKK-JISYO.akaza",
                    ]
                    .iter()
                    .all(|name| {
                        self.pack_asset(language, &format!("akaza-default-model/{name}"))
                            .is_some_and(|path| path.is_file())
                    })
            }
        }
    }

    pub fn needs_update(&self, language: Language) -> bool {
        self.is_installed(language)
            && matches!(language, Language::Japanese)
            && !self.supports_live_input(language)
    }

    pub fn glyph(&self, character: char) -> Option<&PackedGlyph> {
        self.packs
            .values()
            .find_map(|pack| pack.glyphs.get(&character))
    }

    pub fn install(&mut self, language: Language) -> Result<(), LanguageError> {
        if language.is_builtin() || (self.is_installed(language) && !self.needs_update(language)) {
            return Ok(());
        }

        if let Some(source) = env::var_os("TERMLEAF_LANGUAGE_PACK_SOURCE") {
            return self
                .install_from_source(language, &PathBuf::from(source).join(language.code()));
        }

        fs::create_dir_all(&self.data_dir).map_err(|error| {
            LanguageError(format!("cannot create language data directory: {error}"))
        })?;
        let temporary = self.data_dir.join(format!(
            ".install-{}-{}-{}",
            language.code(),
            std::process::id(),
            nonce()
        ));
        fs::create_dir_all(&temporary)
            .map_err(|error| LanguageError(format!("cannot stage language pack: {error}")))?;

        let archive_name = format!("termleaf-language-{}.tar.xz", language.code());
        let archive = temporary.join(&archive_name);
        let checksum = temporary.join(format!("{archive_name}.sha256"));
        let base = env::var("TERMLEAF_LANGUAGE_PACK_URL")
            .unwrap_or_else(|_| format!("{RELEASE_BASE}/v{}", env!("CARGO_PKG_VERSION")));

        let result = (|| {
            download(&format!("{base}/{archive_name}"), &archive)?;
            download(&format!("{base}/{archive_name}.sha256"), &checksum)?;
            verify_checksum(&archive, &checksum)?;
            run_command(
                Command::new("tar")
                    .arg("-xJf")
                    .arg(&archive)
                    .arg("-C")
                    .arg(&temporary),
                "extract language pack",
            )?;
            let extracted = temporary.join(format!("termleaf-language-{}", language.code()));
            self.install_from_source(language, &extracted)
        })();

        let _ = fs::remove_dir_all(&temporary);
        result
    }

    pub fn install_from_source(
        &mut self,
        language: Language,
        source: &Path,
    ) -> Result<(), LanguageError> {
        if language.is_builtin() {
            return Ok(());
        }
        let pack = load_pack(source, language)?;
        fs::create_dir_all(self.languages_dir()).map_err(|error| {
            LanguageError(format!("cannot create languages directory: {error}"))
        })?;
        let destination = self.pack_dir(language);
        let staging = self
            .languages_dir()
            .join(format!(".{}-{}", language.code(), nonce()));
        copy_pack(source, &staging)?;
        if destination.exists() {
            fs::remove_dir_all(&destination).map_err(|error| {
                LanguageError(format!("cannot replace existing language pack: {error}"))
            })?;
        }
        fs::rename(&staging, &destination)
            .map_err(|error| LanguageError(format!("cannot activate language pack: {error}")))?;
        self.packs.insert(language, pack);
        Ok(())
    }

    pub fn remove(&mut self, language: Language) -> Result<(), LanguageError> {
        if language.is_builtin() {
            return Err(LanguageError("English is built into Termleaf".into()));
        }
        let path = self.pack_dir(language);
        if path.exists() {
            fs::remove_dir_all(path)
                .map_err(|error| LanguageError(format!("cannot remove language pack: {error}")))?;
        }
        self.packs.remove(&language);
        Ok(())
    }

    pub fn pack_asset(&self, language: Language, name: &str) -> Option<PathBuf> {
        let path = self.pack_dir(language).join(name);
        path.exists().then_some(path)
    }

    fn languages_dir(&self) -> PathBuf {
        self.data_dir.join("languages")
    }

    fn pack_dir(&self, language: Language) -> PathBuf {
        self.languages_dir().join(language.code())
    }
}

fn data_dir() -> PathBuf {
    if let Some(path) = env::var_os("TERMLEAF_DATA_HOME") {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("termleaf");
    }
    env::var_os("HOME").map_or_else(
        || PathBuf::from(".termleaf-data"),
        |home| PathBuf::from(home).join(".local/share/termleaf"),
    )
}

fn load_pack(path: &Path, expected: Language) -> Result<LanguagePack, LanguageError> {
    let manifest = fs::read_to_string(path.join("manifest.txt"))
        .map_err(|error| LanguageError(format!("cannot read language manifest: {error}")))?;
    let id = manifest_value(&manifest, "id")
        .ok_or_else(|| LanguageError("language manifest is missing id".into()))?;
    let schema = manifest_value(&manifest, "schema")
        .ok_or_else(|| LanguageError("language manifest is missing schema".into()))?;
    if id != expected.code() || schema != "1" {
        return Err(LanguageError(format!(
            "incompatible language pack: expected {} schema 1",
            expected.code()
        )));
    }
    let bytes = fs::read(path.join("glyphs.bin"))
        .map_err(|error| LanguageError(format!("cannot read language glyphs: {error}")))?;
    Ok(LanguagePack {
        glyphs: parse_glyphs(&bytes)?,
    })
}

fn manifest_value<'a>(manifest: &'a str, key: &str) -> Option<&'a str> {
    manifest.lines().find_map(|line| {
        let (candidate, value) = line.split_once('=')?;
        (candidate.trim() == key).then(|| value.trim())
    })
}

fn parse_glyphs(bytes: &[u8]) -> Result<BTreeMap<char, PackedGlyph>, LanguageError> {
    if bytes.len() < 9 || &bytes[..5] != PACK_MAGIC {
        return Err(LanguageError(
            "language glyph file has an invalid header".into(),
        ));
    }
    let count = u32::from_be_bytes(bytes[5..9].try_into().expect("fixed count width")) as usize;
    let expected = 9 + count * GLYPH_RECORD_SIZE;
    if bytes.len() != expected {
        return Err(LanguageError(format!(
            "language glyph file has {} bytes; expected {expected}",
            bytes.len()
        )));
    }

    let mut glyphs = BTreeMap::new();
    for record in bytes[9..].chunks_exact(GLYPH_RECORD_SIZE) {
        let codepoint = u32::from_be_bytes(record[..4].try_into().expect("fixed codepoint width"));
        let character = char::from_u32(codepoint)
            .ok_or_else(|| LanguageError(format!("invalid glyph codepoint U+{codepoint:04X}")))?;
        let width = record[4] as usize;
        if !(1..=16).contains(&width) {
            return Err(LanguageError(format!("invalid glyph width {width}")));
        }
        let mut rows = [0; GLYPH_ROWS];
        for (index, row) in record[5..].chunks_exact(2).enumerate() {
            rows[index] = u16::from_be_bytes([row[0], row[1]]);
        }
        glyphs.insert(character, PackedGlyph { width, rows });
    }
    Ok(glyphs)
}

fn copy_pack(source: &Path, destination: &Path) -> Result<(), LanguageError> {
    fs::create_dir_all(destination)
        .map_err(|error| LanguageError(format!("cannot stage language pack: {error}")))?;
    copy_pack_entries(source, destination)
}

fn copy_pack_entries(source: &Path, destination: &Path) -> Result<(), LanguageError> {
    for entry in fs::read_dir(source)
        .map_err(|error| LanguageError(format!("cannot read language pack: {error}")))?
    {
        let entry = entry
            .map_err(|error| LanguageError(format!("cannot inspect language pack: {error}")))?;
        let from = entry.path();
        if from.is_file() {
            fs::copy(&from, destination.join(entry.file_name())).map_err(|error| {
                LanguageError(format!("cannot copy {}: {error}", from.display()))
            })?;
        } else if from.is_dir() {
            let child_destination = destination.join(entry.file_name());
            fs::create_dir_all(&child_destination).map_err(|error| {
                LanguageError(format!(
                    "cannot create {}: {error}",
                    child_destination.display()
                ))
            })?;
            copy_pack_entries(&from, &child_destination)?;
        }
    }
    Ok(())
}

fn download(url: &str, destination: &Path) -> Result<(), LanguageError> {
    run_command(
        Command::new("curl")
            .args(["--proto", "=https", "--tlsv1.2", "-LsSf", url, "-o"])
            .arg(destination),
        "download language pack",
    )
}

fn verify_checksum(archive: &Path, checksum: &Path) -> Result<(), LanguageError> {
    let expected = fs::read_to_string(checksum)
        .map_err(|error| LanguageError(format!("cannot read language checksum: {error}")))?
        .split_whitespace()
        .next()
        .ok_or_else(|| LanguageError("language checksum is empty".into()))?
        .to_owned();
    let output = if Command::new("sha256sum").arg(archive).output().is_ok() {
        Command::new("sha256sum").arg(archive).output()
    } else {
        Command::new("shasum")
            .args(["-a", "256"])
            .arg(archive)
            .output()
    }
    .map_err(|error| LanguageError(format!("cannot calculate language checksum: {error}")))?;
    if !output.status.success() {
        return Err(LanguageError("language checksum command failed".into()));
    }
    let actual = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned();
    if actual != expected {
        return Err(LanguageError(
            "language pack checksum verification failed".into(),
        ));
    }
    Ok(())
}

fn run_command(command: &mut Command, action: &str) -> Result<(), LanguageError> {
    let status = command
        .status()
        .map_err(|error| LanguageError(format!("cannot {action}: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(LanguageError(format!("could not {action}: {status}")))
    }
}

fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_data_dir(name: &str) -> PathBuf {
        env::temp_dir().join(format!("termleaf-language-{name}-{}", nonce()))
    }

    #[test]
    fn installs_and_removes_data_only_language_packs() {
        let root = unique_data_dir("lifecycle");
        let mut registry = LanguageRegistry::load_from(root.clone());
        assert!(registry.is_installed(Language::English));
        assert!(!registry.is_installed(Language::Korean));

        registry
            .install_from_source(Language::Korean, Path::new("language-packs/ko"))
            .unwrap();
        assert!(registry.is_installed(Language::Korean));
        assert!(registry.supports_live_input(Language::Korean));
        assert_eq!(registry.glyph('한').map(|glyph| glyph.width), Some(10));

        registry.remove(Language::Korean).unwrap();
        assert!(!registry.is_installed(Language::Korean));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn japanese_pack_contains_kana_and_cjk_glyphs() {
        let pack = load_pack(Path::new("language-packs/ja"), Language::Japanese).unwrap();
        for character in ['あ', 'ア', '日', '本', '語'] {
            assert!(pack.glyphs.contains_key(&character), "missing {character}");
        }
    }

    #[test]
    fn japanese_pack_without_conversion_model_requires_update() {
        let root = unique_data_dir("stale-japanese");
        let mut registry = LanguageRegistry::load_from(root.clone());
        registry
            .install_from_source(Language::Japanese, Path::new("language-packs/ja"))
            .unwrap();

        assert!(registry.is_installed(Language::Japanese));
        assert!(!registry.supports_live_input(Language::Japanese));
        assert!(registry.needs_update(Language::Japanese));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn install_from_source_copies_extra_language_pack_assets() {
        let root = unique_data_dir("extra-assets");
        let source = root.join("source");
        fs::create_dir_all(&source).unwrap();
        fs::copy(
            "language-packs/ja/manifest.txt",
            source.join("manifest.txt"),
        )
        .unwrap();
        fs::copy("language-packs/ja/glyphs.bin", source.join("glyphs.bin")).unwrap();
        fs::write(source.join("LICENSE"), "license").unwrap();
        fs::create_dir_all(source.join("akaza-default-model")).unwrap();
        for name in [
            "unigram.model",
            "bigram.model",
            "skip_bigram.model",
            "SKK-JISYO.akaza",
        ] {
            fs::write(source.join("akaza-default-model").join(name), "fixture").unwrap();
        }

        let mut registry = LanguageRegistry::load_from(root.clone());
        registry
            .install_from_source(Language::Japanese, &source)
            .unwrap();
        assert!(registry.supports_live_input(Language::Japanese));
        assert!(!registry.needs_update(Language::Japanese));

        let copied = registry
            .pack_asset(Language::Japanese, "akaza-default-model/unigram.model")
            .expect("extra asset should be available");
        assert_eq!(fs::read_to_string(copied).unwrap(), "fixture");

        let _ = fs::remove_dir_all(root);
    }
}
