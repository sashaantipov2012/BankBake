use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BakeMode {
    Modding,
    Standalone,
}

impl Default for BakeMode {
    fn default() -> Self {
        Self::Standalone
    }
}

fn default_true() -> bool {
    true
}

fn default_is_debug_build() -> bool {
    cfg!(debug_assertions)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub mode: BakeMode,
    #[serde(default)]
    pub fmod_compiler_path: Option<String>,
    #[serde(default)]
    pub output_directory: Option<String>,
    #[serde(default = "default_true")]
    pub build_master_banks: bool,
    #[serde(default = "default_true")]
    pub export_guids: bool,
    #[serde(default = "default_true")]
    pub export_project_file: bool,
    #[serde(default)]
    pub keep_temp_project: bool,
    #[serde(default)]
    pub suppress_update_popup: bool,
    #[serde(default = "default_is_debug_build", skip_deserializing)]
    pub is_debug_build: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: BakeMode::Standalone,
            fmod_compiler_path: None,
            output_directory: None,
            build_master_banks: true,
            export_guids: true,
            export_project_file: true,
            keep_temp_project: false,
            suppress_update_popup: false,
            is_debug_build: cfg!(debug_assertions),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerInfo {
    pub available: bool,
    pub bundled_available: bool,
    pub path: Option<String>,
    pub source: String, // "configured", "bundled", "system", "wine", "not_found"
    pub version: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFileInfo {
    pub path: String,
    pub filename: String,
    pub suggested_event_name: String,
    pub size_bytes: u64,
    pub extension: String,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum EventCategory {
    Sfx,
    Music,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BakeEventItem {
    pub id: String,
    pub file_path: String,
    pub filename: String,
    pub event_name: String,
    pub category: EventCategory,
    pub looping: bool,
    pub volume_db: f64,
    pub custom_loop_enabled: bool,
    pub loop_start: f64,
    pub loop_end: Option<f64>,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BakeBankRequest {
    pub bank_name: String,
    #[serde(default)]
    pub master_bank_name: Option<String>,
    pub events: Vec<BakeEventItem>,
    pub mode: Option<BakeMode>,
    pub output_directory: Option<String>,
    pub fmod_compiler_path: Option<String>,
    pub build_master_banks: Option<bool>,
    pub export_guids: Option<bool>,
    pub export_project_file: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEventEntry {
    pub id: String,
    pub archive_path: String,
    pub filename: String,
    pub event_name: String,
    pub category: EventCategory,
    pub looping: bool,
    pub volume_db: f64,
    pub custom_loop_enabled: bool,
    pub loop_start: f64,
    pub loop_end: Option<f64>,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankBakeProjectManifest {
    pub format: String,
    pub version: String,
    pub bank_name: String,
    #[serde(default)]
    pub master_bank_name: Option<String>,
    pub events: Vec<ProjectEventEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedProjectEvent {
    pub id: String,
    pub file_path: String,
    pub filename: String,
    pub event_name: String,
    pub category: EventCategory,
    pub looping: bool,
    pub volume_db: f64,
    pub custom_loop_enabled: bool,
    pub loop_start: f64,
    pub loop_end: Option<f64>,
    pub duration: f64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedProjectData {
    pub bank_name: String,
    pub master_bank_name: String,
    pub project_path: String,
    pub events: Vec<LoadedProjectEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltFileEntry {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BakeBankResult {
    pub success: bool,
    pub bank_name: String,
    pub output_directory: String,
    pub built_files: Vec<BuiltFileEntry>,
    pub guids_content: Option<String>,
    pub compiler_stdout: String,
    pub compiler_stderr: String,
    pub generated_script: String,
    pub duration_ms: u64,
    pub message: String,
}

/// Sanitize a filename stem into a clean FMOD event name
pub fn sanitize_event_name(stem: &str) -> String {
    let mut out = String::with_capacity(stem.len());
    for c in stem.chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
        } else if c.is_whitespace() || c == '.' {
            if !out.ends_with('_') {
                out.push('_');
            }
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "untitled_event".to_string()
    } else {
        trimmed
    }
}

/// Sanitize a bank name
pub fn sanitize_bank_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Bank name cannot be empty.".to_string());
    }
    let forbidden = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if trimmed.chars().any(|c| forbidden.contains(&c)) {
        return Err("Bank name contains invalid characters (< > : \" / \\ | ? *).".to_string());
    }
    Ok(trimmed.to_string())
}

/// Find the bundled Windows CLI relative to the app or workspace.
fn find_bundled_windows_cli() -> Option<PathBuf> {
    let mut candidate_roots: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidate_roots.push(cwd.join("fmodstudio"));
        if let Some(parent) = cwd.parent() {
            candidate_roots.push(parent.join("fmodstudio"));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidate_roots.push(exe_dir.join("fmodstudio"));
            if let Some(p1) = exe_dir.parent() {
                candidate_roots.push(p1.join("fmodstudio"));
                if let Some(p2) = p1.parent() {
                    candidate_roots.push(p2.join("fmodstudio"));
                    if let Some(p3) = p2.parent() {
                        candidate_roots.push(p3.join("fmodstudio"));
                    }
                }
            }
        }
    }
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        let m_path = PathBuf::from(manifest_dir);
        candidate_roots.push(m_path.join("fmodstudio"));
        if let Some(parent) = m_path.parent() {
            candidate_roots.push(parent.join("fmodstudio"));
        }
    }

    for root in candidate_roots {
        let cl = root.join("fmodstudiocl.exe");
        if cl.exists() && cl.is_file() {
            return Some(cl);
        }
    }
    None
}

/// Find the bundled compiler that takes precedence over custom paths in release builds.
pub fn find_bundled_compiler() -> Option<PathBuf> {
    #[cfg(not(target_os = "linux"))]
    return find_bundled_windows_cli();
    #[cfg(target_os = "linux")]
    return None;
}

#[cfg(not(target_os = "linux"))]
pub fn discover_compiler(custom_path: Option<&str>) -> CompilerInfo {
    let bundled_exe = find_bundled_compiler();
    let bundled_available = bundled_exe.is_some();

    // 1. Check custom configured path if in debug mode OR if bundled compiler is missing
    if cfg!(debug_assertions) || !bundled_available {
        if let Some(cp) = custom_path {
            let trimmed = cp.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                let candidate = if p.is_dir() {
                    let cl = p.join("fmodstudiocl.exe");
                    if cl.exists() {
                        cl
                    } else {
                        p.join("fmodstudio.exe")
                    }
                } else {
                    p
                };

                if candidate.exists() && candidate.is_file() {
                    return CompilerInfo {
                        available: true,
                        bundled_available,
                        path: Some(candidate.to_string_lossy().to_string()),
                        source: "configured".to_string(),
                        version: Some("FMOD Studio 2.02".to_string()),
                        message: format!("Using configured FMOD compiler at {}", candidate.display()),
                    };
                }
            }
        }
    }

    // 2. Check bundled fmodstudio folder
    if let Some(cl) = bundled_exe {
        return CompilerInfo {
            available: true,
            bundled_available: true,
            path: Some(cl.to_string_lossy().to_string()),
            source: "bundled".to_string(),
            version: Some("FMOD Studio 2.02.35".to_string()),
            message: format!("Bundled FMOD CLI Ready ({})", cl.display()),
        };
    }

    // 3. Check standard system installation paths
    let system_paths = [
        "C:\\Program Files\\FMOD SoundSystem\\FMOD Studio 2.02.00\\fmodstudiocl.exe",
        "C:\\Program Files\\FMOD SoundSystem\\FMOD Studio 2.03.00\\fmodstudiocl.exe",
        "C:\\Program Files (x86)\\FMOD SoundSystem\\FMOD Studio 2.02.00\\fmodstudiocl.exe",
    ];
    for sp in system_paths {
        let p = PathBuf::from(sp);
        if p.exists() && p.is_file() {
            return CompilerInfo {
                available: true,
                bundled_available: false,
                path: Some(p.to_string_lossy().to_string()),
                source: "system".to_string(),
                version: Some("FMOD Studio 2.x".to_string()),
                message: format!("System FMOD CLI found at {}", p.display()),
            };
        }
    }

    // Check Program Files\FMOD SoundSystem subdirectories dynamically
    let prog_fmod = PathBuf::from("C:\\Program Files\\FMOD SoundSystem");
    if prog_fmod.exists() && prog_fmod.is_dir() {
        if let Ok(entries) = fs::read_dir(&prog_fmod) {
            for entry in entries.flatten() {
                let cl = entry.path().join("fmodstudiocl.exe");
                if cl.exists() && cl.is_file() {
                    return CompilerInfo {
                        available: true,
                        bundled_available: false,
                        path: Some(cl.to_string_lossy().to_string()),
                        source: "system".to_string(),
                        version: Some("FMOD Studio 2.x".to_string()),
                        message: format!("System FMOD CLI found at {}", cl.display()),
                    };
                }
            }
        }
    }

    CompilerInfo {
        available: false,
        bundled_available: false,
        path: None,
        source: "not_found".to_string(),
        version: None,
        message: "FMOD Studio CLI (fmodstudiocl.exe) not found. Configure path in Settings.".to_string(),
    }
}

#[cfg(target_os = "linux")]
fn linux_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(target_os = "linux")]
fn executable_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| linux_executable(candidate))
}

#[cfg(target_os = "linux")]
fn is_windows_executable(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
}

#[cfg(target_os = "linux")]
pub fn discover_compiler(custom_path: Option<&str>) -> CompilerInfo {
    let wine_available = cfg!(target_arch = "x86_64")
        && executable_on_path("wine").is_some()
        && executable_on_path("winepath").is_some();

    let configured = custom_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .and_then(|path| {
            let path = std::path::absolute(path).ok()?;
            let candidates = if path.is_dir() {
                vec![path.join("fmodstudiocl"), path.join("fmodstudiocl.exe")]
            } else {
                vec![path]
            };
            candidates.into_iter().find(|path| {
                if is_windows_executable(path) {
                    wine_available && path.is_file()
                } else {
                    linux_executable(path)
                }
            })
        });

    let compiler = configured
        .map(|path| {
            let source = if is_windows_executable(&path) {
                "wine"
            } else {
                "configured"
            };
            (path, source, None)
        })
        .or_else(|| {
            executable_on_path("fmodstudiocl")
                .or_else(|| {
                    let path = PathBuf::from("/opt/fmodstudio/fmodstudiocl");
                    linux_executable(&path).then_some(path)
                })
                .map(|path| (path, "system", None))
        })
        .or_else(|| {
            if wine_available {
                find_bundled_windows_cli().map(|path| (path, "wine", Some("FMOD Studio 2.02.35")))
            } else {
                None
            }
        });

    if let Some((path, source, version)) = compiler {
        return CompilerInfo {
            available: true,
            bundled_available: false,
            path: Some(path.to_string_lossy().into_owned()),
            source: source.to_string(),
            version: version.map(str::to_string),
            message: format!("FMOD Studio CLI found at {}", path.display()),
        };
    }

    let mut message = "FMOD Studio CLI (fmodstudiocl) not found. Install FMOD Studio or set the CLI path in Settings.".to_string();
    if cfg!(target_arch = "x86_64") {
        message.push_str(" Install Wine to use the bundled Windows CLI.");
    }
    CompilerInfo {
        available: false,
        bundled_available: false,
        path: None,
        source: "not_found".to_string(),
        version: None,
        message,
    }
}

#[cfg(target_os = "linux")]
fn wine_workspace_path(path: &Path) -> Result<String, String> {
    let output = Command::new("winepath")
        .args(["-w", "--"])
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to run winepath: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "winepath could not map {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mapped = String::from_utf8_lossy(&output.stdout);
    let mapped = mapped.trim_end_matches(['\r', '\n']);
    if mapped.is_empty() {
        return Err(format!("winepath returned no path for {}", path.display()));
    }
    // Keep Wine's drive mapping and use forward slashes for FMOD's Qt file APIs.
    Ok(mapped.replace('\\', "/"))
}

/// Fast WAV/OGG header duration probing fallback so backend can also report duration if available
pub fn probe_audio_duration(path: &Path) -> Option<f64> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "wav" {
        let bytes = fs::read(path).ok()?;
        if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return None;
        }
        let mut pos = 12;
        let mut byte_rate: u32 = 0;
        let mut data_size: u32 = 0;
        while pos + 8 <= bytes.len() {
            let chunk_id = &bytes[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]) as usize;
            if chunk_id == b"fmt " && chunk_size >= 16 && pos + 8 + 16 <= bytes.len() {
                byte_rate = u32::from_le_bytes([
                    bytes[pos + 16],
                    bytes[pos + 17],
                    bytes[pos + 18],
                    bytes[pos + 19],
                ]);
            } else if chunk_id == b"data" {
                data_size = chunk_size as u32;
                break;
            }
            pos += 8 + chunk_size + (chunk_size % 2);
        }
        if byte_rate > 0 && data_size > 0 {
            return Some(data_size as f64 / byte_rate as f64);
        }
    } else if ext == "ogg" {
        let bytes = fs::read(path).ok()?;
        if bytes.len() > 28 && &bytes[0..4] == b"OggS" {
            // In Vorbis first identification header: sample rate is at offset 28 + 12 = 40 (u32 LE)
            if bytes.len() >= 44 && &bytes[29..35] == b"vorbis" {
                let sample_rate =
                    u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);
                if sample_rate > 0 {
                    // Find last OggS page granule position
                    let search_start = bytes.len().saturating_sub(65536);
                    let tail = &bytes[search_start..];
                    for i in (0..tail.len().saturating_sub(14)).rev() {
                        if &tail[i..i + 4] == b"OggS" {
                            let granule = u64::from_le_bytes([
                                tail[i + 6],
                                tail[i + 7],
                                tail[i + 8],
                                tail[i + 9],
                                tail[i + 10],
                                tail[i + 11],
                                tail[i + 12],
                                tail[i + 13],
                            ]);
                            if granule > 0 && granule != u64::MAX {
                                return Some(granule as f64 / sample_rate as f64);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn inspect_audio_file(path_str: &str) -> Result<AudioFileInfo, String> {
    let path = PathBuf::from(path_str);
    if !path.exists() || !path.is_file() {
        return Err(format!("Audio file does not exist: {}", path_str));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let allowed = ["wav", "ogg", "mp3", "flac", "aif", "aiff", "wma"];
    if !allowed.contains(&ext.as_str()) {
        return Err(format!(
            "Unsupported audio format '.{}'. Supported formats: WAV, OGG, MP3, FLAC, AIFF, WMA.",
            ext
        ));
    }
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio")
        .to_string();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("event");
    let suggested_event_name = sanitize_event_name(stem);
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    let duration_seconds = probe_audio_duration(&path);

    Ok(AudioFileInfo {
        path: path.to_string_lossy().to_string(),
        filename,
        suggested_event_name,
        size_bytes: meta.len(),
        extension: ext,
        duration_seconds,
    })
}

/// Generate the minimal valid FMOD Studio 2.02 skeleton project in `project_dir`
pub fn create_temp_fmod_project(project_dir: &Path, master_bank_name: &str) -> Result<PathBuf, String> {
    let meta_dir = project_dir.join("Metadata");
    let subdirs = [
        project_dir.join("Assets"),
        meta_dir.clone(),
        meta_dir.join("Asset"),
        meta_dir.join("AudioFile"),
        meta_dir.join("Bank"),
        meta_dir.join("BankFolder"),
        meta_dir.join("EffectPresetFolder"),
        meta_dir.join("EncodingSetting"),
        meta_dir.join("Event"),
        meta_dir.join("EventFolder"),
        meta_dir.join("Group"),
        meta_dir.join("ParameterPresetFolder"),
        meta_dir.join("Platform"),
        meta_dir.join("ProfilerFolder"),
        meta_dir.join("SandboxFolder"),
        meta_dir.join("SnapshotGroup"),
    ];

    for d in &subdirs {
        fs::create_dir_all(d).map_err(|e| format!("Failed to create directory {}: {}", d.display(), e))?;
    }

    let fspro_path = project_dir.join("BankBakeProject.fspro");
    fs::write(
        &fspro_path,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00" />
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir.join("Workspace.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="Workspace" id="{eca33bb7-c1b3-4fe2-9fef-b7789ec3bde5}">
		<property name="builtBanksOutputDirectory">
			<value>Build</value>
		</property>
		<relationship name="masterEventFolder">
			<destination>{10626052-c98b-4665-b391-646f2512cecd}</destination>
		</relationship>
		<relationship name="masterTagFolder">
			<destination>{1b486b69-30e1-4779-90d4-f45386aa7367}</destination>
		</relationship>
		<relationship name="masterEffectPresetFolder">
			<destination>{aca7fd17-4939-0ce5-284c-74c26c487e44}</destination>
		</relationship>
		<relationship name="masterParameterPresetFolder">
			<destination>{a418e0de-c7d0-05c9-280a-b529dc02e12e}</destination>
		</relationship>
		<relationship name="masterBankFolder">
			<destination>{07fb52a7-88b7-4b50-aff5-5f9728462f36}</destination>
		</relationship>
		<relationship name="masterSandboxFolder">
			<destination>{90f1921b-3a6d-4222-bafe-83641a15bccf}</destination>
		</relationship>
		<relationship name="masterAssetFolder">
			<destination>{4c43130b-9186-4fb6-9d65-a9acdac70927}</destination>
		</relationship>
		<relationship name="mixer">
			<destination>{204ed808-83ea-42f4-b502-9b293dd2b6ba}</destination>
		</relationship>
		<relationship name="profilerSessionFolder">
			<destination>{26fc004a-e651-4c80-ac8f-6688d3221c0a}</destination>
		</relationship>
		<relationship name="platforms">
			<destination>{c9f35dc9-139f-472b-8888-8ad9fad5168c}</destination>
		</relationship>
	</object>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir.join("Master.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="MixerMaster" id="{b78fd6e5-45cb-4125-915b-e8841bf2976e}">
		<property name="name">
			<value>Master Bus</value>
		</property>
		<relationship name="effectChain">
			<destination>{92bdd82e-c037-4218-9180-212628eb7e62}</destination>
		</relationship>
		<relationship name="panner">
			<destination>{f5190665-2c6e-4635-bf36-1e39c167ee36}</destination>
		</relationship>
		<relationship name="mixer">
			<destination>{204ed808-83ea-42f4-b502-9b293dd2b6ba}</destination>
		</relationship>
	</object>
	<object class="MixerBusEffectChain" id="{92bdd82e-c037-4218-9180-212628eb7e62}">
		<relationship name="effects">
			<destination>{93621478-1067-4bca-9c53-fc6d7d738a70}</destination>
		</relationship>
	</object>
	<object class="MixerBusPanner" id="{f5190665-2c6e-4635-bf36-1e39c167ee36}">
		<property name="overridingOutputFormat">
			<value>2</value>
		</property>
	</object>
	<object class="MixerBusFader" id="{93621478-1067-4bca-9c53-fc6d7d738a70}" />
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir.join("Mixer.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
    <object class="Mixer" id="{204ed808-83ea-42f4-b502-9b293dd2b6ba}">
        <relationship name="masterBus">
            <destination>{b78fd6e5-45cb-4125-915b-e8841bf2976e}</destination>
        </relationship>
        <relationship name="snapshotList">
            <destination>{2e15ae1f-9888-48d4-9994-adc998ae281e}</destination>
        </relationship>
    </object>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    // Pre-create Music and SFX MixerGroups with Pizza Tower / standard FMOD bus GUIDs
    fs::write(
        meta_dir
            .join("Group")
            .join("{032f56ee-6215-4738-8d0b-f630b5c73daf}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="MixerGroup" id="{032f56ee-6215-4738-8d0b-f630b5c73daf}">
		<property name="name">
			<value>Music</value>
		</property>
		<relationship name="effectChain">
			<destination>{dbc14ad0-a58d-478c-9e10-5df229e206c3}</destination>
		</relationship>
		<relationship name="panner">
			<destination>{bfbace69-aa45-4e7f-b2ca-ef8f4c761769}</destination>
		</relationship>
		<relationship name="output">
			<destination>{b78fd6e5-45cb-4125-915b-e8841bf2976e}</destination>
		</relationship>
	</object>
	<object class="MixerBusEffectChain" id="{dbc14ad0-a58d-478c-9e10-5df229e206c3}">
		<relationship name="effects">
			<destination>{7fd423d2-f491-4d97-9ada-841c9377334e}</destination>
		</relationship>
	</object>
	<object class="MixerBusPanner" id="{bfbace69-aa45-4e7f-b2ca-ef8f4c761769}" />
	<object class="MixerBusFader" id="{7fd423d2-f491-4d97-9ada-841c9377334e}" />
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("Group")
            .join("{f18d5877-3d50-41c0-b6c0-9be04ada9938}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="MixerGroup" id="{f18d5877-3d50-41c0-b6c0-9be04ada9938}">
		<property name="name">
			<value>SFX</value>
		</property>
		<relationship name="effectChain">
			<destination>{fd2b0385-e5e7-4c14-8348-7ada37283192}</destination>
		</relationship>
		<relationship name="panner">
			<destination>{e0d8b0f4-1b05-4412-9cdd-9994818597ad}</destination>
		</relationship>
		<relationship name="output">
			<destination>{b78fd6e5-45cb-4125-915b-e8841bf2976e}</destination>
		</relationship>
	</object>
	<object class="MixerBusEffectChain" id="{fd2b0385-e5e7-4c14-8348-7ada37283192}">
		<relationship name="effects">
			<destination>{a7a9dc9e-2051-44f7-8925-0ddb318ebacb}</destination>
		</relationship>
	</object>
	<object class="MixerBusPanner" id="{e0d8b0f4-1b05-4412-9cdd-9994818597ad}" />
	<object class="MixerBusFader" id="{a7a9dc9e-2051-44f7-8925-0ddb318ebacb}" />
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir.join("Tags.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="MasterTagFolder" id="{1b486b69-30e1-4779-90d4-f45386aa7367}">
		<property name="name">
			<value>Master</value>
		</property>
	</object>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("Asset")
            .join("{4c43130b-9186-4fb6-9d65-a9acdac70927}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
    <object class="MasterAssetFolder" id="{4c43130b-9186-4fb6-9d65-a9acdac70927}"/>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("BankFolder")
            .join("{07fb52a7-88b7-4b50-aff5-5f9728462f36}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
    <object class="MasterBankFolder" id="{07fb52a7-88b7-4b50-aff5-5f9728462f36}"/>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    // Generate a unique UUID for the Master Bank object on every build so that
    // Master.bank and Master.strings.bank from two different mods never share
    // the same internal bank GUID (preventing FMOD_ERR_EVENT_ALREADY_LOADED).
    let master_bank_guid = format!("{{{}}}", Uuid::new_v4());
    let xml_safe_master_name = master_bank_name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    fs::write(
        meta_dir
            .join("Bank")
            .join(format!("{}.xml", master_bank_guid)),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="Bank" id="{}">
		<property name="name">
			<value>{}</value>
		</property>
		<property name="isMasterBank">
			<value>true</value>
		</property>
		<relationship name="folder">
			<destination>{{07fb52a7-88b7-4b50-aff5-5f9728462f36}}</destination>
		</relationship>
	</object>
</objects>
"#,
            master_bank_guid, xml_safe_master_name
        ),
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("EffectPresetFolder")
            .join("{aca7fd17-4939-0ce5-284c-74c26c487e44}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
    <object class="MasterEffectPresetFolder" id="{aca7fd17-4939-0ce5-284c-74c26c487e44}"/>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("ParameterPresetFolder")
            .join("{a418e0de-c7d0-05c9-280a-b529dc02e12e}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
    <object class="MasterParameterPresetFolder" id="{a418e0de-c7d0-05c9-280a-b529dc02e12e}"/>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("EventFolder")
            .join("{10626052-c98b-4665-b391-646f2512cecd}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
    <object class="MasterEventFolder" id="{10626052-c98b-4665-b391-646f2512cecd}">
        <property name="name">
            <value>Master</value>
        </property>
    </object>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("Platform")
            .join("{c9f35dc9-139f-472b-8888-8ad9fad5168c}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
    <object class="Platform" id="{c9f35dc9-139f-472b-8888-8ad9fad5168c}">
        <property name="hardwareType">
            <value>0</value>
        </property>
        <property name="name">
            <value>Desktop</value>
        </property>
        <property name="subDirectory">
            <value>Desktop</value>
        </property>
        <property name="speakerFormat">
            <value>5</value>
        </property>
    </object>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("EncodingSetting")
            .join("{fa3463e9-0731-437c-8bee-4e68a4d3ffea}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
    <object class="EncodingSetting" id="{fa3463e9-0731-437c-8bee-4e68a4d3ffea}">
        <property name="encodingFormat">
            <value>3</value>
        </property>
        <property name="quality">
            <value>37</value>
        </property>
        <relationship name="platform">
            <destination>{c9f35dc9-139f-472b-8888-8ad9fad5168c}</destination>
        </relationship>
        <relationship name="encodable">
            <destination>{c9f35dc9-139f-472b-8888-8ad9fad5168c}</destination>
        </relationship>
    </object>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("ProfilerFolder")
            .join("{26fc004a-e651-4c80-ac8f-6688d3221c0a}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="ProfilerSessionFolder" id="{26fc004a-e651-4c80-ac8f-6688d3221c0a}" />
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("SandboxFolder")
            .join("{90f1921b-3a6d-4222-bafe-83641a15bccf}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="MasterSandboxFolder" id="{90f1921b-3a6d-4222-bafe-83641a15bccf}" />
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    fs::write(
        meta_dir
            .join("SnapshotGroup")
            .join("{2e15ae1f-9888-48d4-9994-adc998ae281e}.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<objects serializationModel="Studio.02.02.00">
	<object class="SnapshotList" id="{2e15ae1f-9888-48d4-9994-adc998ae281e}">
		<relationship name="mixer">
			<destination>{204ed808-83ea-42f4-b502-9b293dd2b6ba}</destination>
		</relationship>
	</object>
</objects>
"#,
    )
    .map_err(|e| e.to_string())?;

    Ok(fspro_path)
}

fn escape_js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push('/'),
            '"' => out.push_str("\\\""),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\u{2028}' | '\u{2029}' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Parse an event path like `event:/sfx/player/jump`, `/sfx/player/jump`, or `jump`
/// into `(folder_segments, leaf_event_name)`.
/// If no folder segments are present, falls back to `"music"` or `"sfx"` based on `category`.
pub fn parse_event_hierarchy(raw_event_name: &str, category: &EventCategory) -> Result<(Vec<String>, String), String> {
    let mut s = raw_event_name.trim().replace('\\', "/");
    if s.len() >= 7 && s[..7].eq_ignore_ascii_case("event:/") {
        s = s[7..].to_string();
    } else if s.len() >= 6 && s[..6].eq_ignore_ascii_case("event:") {
        s = s[6..].to_string();
    }
    let s = s.trim_matches('/');
    let segments: Vec<String> = s
        .split('/')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect();

    if segments.is_empty() {
        return Err("Event path cannot be empty (e.g. event:/sfx/my_sound).".to_string());
    }

    let forbidden = ['<', '>', ':', '"', '|', '?', '*'];
    for seg in &segments {
        if seg.chars().any(|c| forbidden.contains(&c)) {
            return Err(format!(
                "Event path segment '{}' contains invalid characters (< > : \" | ? *).",
                seg
            ));
        }
    }

    if segments.len() == 1 {
        let default_folder = match category {
            EventCategory::Music => "music".to_string(),
            EventCategory::Sfx => "sfx".to_string(),
        };
        Ok((vec![default_folder], segments[0].clone()))
    } else {
        let leaf = segments.last().unwrap().clone();
        let folders = segments[..segments.len() - 1].to_vec();
        Ok((folders, leaf))
    }
}

/// Generate the FMOD Studio JavaScript automation script for the given bake request
pub fn generate_fmod_bake_script(
    bank_name: &str,
    master_bank_name: &str,
    events: &[BakeEventItem],
    export_guids: bool,
) -> String {
    let mut script = String::new();
    script.push_str("// Generated by BankBake\n");
    script.push_str("(function() {\n");
    script.push_str("    console.log('[BankBake] Initializing FMOD project build...');\n");

    let escaped_bank = escape_js_string(bank_name);
    let escaped_master = escape_js_string(master_bank_name);
    if bank_name.eq_ignore_ascii_case(master_bank_name) {
        script.push_str(&format!(
            "    var targetBank = studio.project.lookup('bank:/{}');\n",
            escaped_master
        ));
    } else {
        script.push_str("    var targetBank = studio.project.create('Bank');\n");
        script.push_str(&format!("    targetBank.name = \"{}\";\n", escaped_bank));
        script.push_str("    targetBank.folder = studio.project.workspace.masterBankFolder;\n");
    }

    script.push_str("    var folderCache = {};\n");
    script.push_str("    function getOrCreateFolderChain(segments) {\n");
    script.push_str("        var currentParent = studio.project.workspace.masterEventFolder;\n");
    script.push_str("        var pathKey = '';\n");
    script.push_str("        for (var i = 0; i < segments.length; i++) {\n");
    script.push_str("            var seg = segments[i];\n");
    script.push_str("            pathKey = pathKey ? (pathKey + '/' + seg.toLowerCase()) : seg.toLowerCase();\n");
    script.push_str("            if (!folderCache[pathKey]) {\n");
    script.push_str("                var f = studio.project.create('EventFolder');\n");
    script.push_str("                f.name = seg;\n");
    script.push_str("                f.folder = currentParent;\n");
    script.push_str("                folderCache[pathKey] = f;\n");
    script.push_str("            }\n");
    script.push_str("            currentParent = folderCache[pathKey];\n");
    script.push_str("        }\n");
    script.push_str("        return currentParent;\n");
    script.push_str("    }\n\n");

    script.push_str("    var musicBus = studio.project.lookup('bus:/Music') || studio.project.lookup('{032f56ee-6215-4738-8d0b-f630b5c73daf}');\n");
    script.push_str("    if (!musicBus) {\n");
    script.push_str("        musicBus = studio.project.create('MixerGroup');\n");
    script.push_str("        musicBus.name = 'Music';\n");
    script.push_str("        musicBus.output = studio.project.workspace.mixer.masterBus;\n");
    script.push_str("    }\n\n");

    script.push_str("    var sfxBus = studio.project.lookup('bus:/SFX') || studio.project.lookup('{f18d5877-3d50-41c0-b6c0-9be04ada9938}');\n");
    script.push_str("    if (!sfxBus) {\n");
    script.push_str("        sfxBus = studio.project.create('MixerGroup');\n");
    script.push_str("        sfxBus.name = 'SFX';\n");
    script.push_str("        sfxBus.output = studio.project.workspace.mixer.masterBus;\n");
    script.push_str("    }\n\n");

    for (idx, item) in events.iter().enumerate() {
        let escaped_path = escape_js_string(&item.file_path);
        let (folders, leaf_name) = parse_event_hierarchy(&item.event_name, &item.category)
            .unwrap_or_else(|_| (vec!["sfx".to_string()], "untitled_event".to_string()));
        let escaped_leaf_name = escape_js_string(&leaf_name);
        let folders_js_array = folders
            .iter()
            .map(|f| format!("\"{}\"", escape_js_string(f)))
            .collect::<Vec<_>>()
            .join(", ");

        let is_music = item.category == EventCategory::Music;
        let bus_var = if is_music { "musicBus" } else { "sfxBus" };

        script.push_str(&format!(
            "    // Event #{}: event:/{}/{}\n",
            idx + 1,
            folders.join("/"),
            escaped_leaf_name
        ));
        script.push_str(&format!(
            "    var asset_{idx} = studio.project.importAudioFile(\"{escaped_path}\");\n"
        ));
        script.push_str(&format!(
            "    if (!asset_{idx}) {{\n        console.error('[BankBake] Failed to import audio file: {escaped_path}');\n    }} else {{\n"
        ));
        if is_music {
            script.push_str(&format!("        asset_{idx}.isStreaming = true;\n"));
        }
        script.push_str(&format!(
            "        var ev_{idx} = studio.project.create('Event');\n"
        ));
        script.push_str(&format!(
            "        ev_{idx}.name = \"{escaped_leaf_name}\";\n"
        ));
        script.push_str(&format!(
            "        ev_{idx}.folder = getOrCreateFolderChain([{folders_js_array}]);\n"
        ));
        script.push_str(&format!(
            "        ev_{idx}.mixerInput.output = {bus_var};\n"
        ));
        script.push_str(&format!(
            "        var trk_{idx} = ev_{idx}.addGroupTrack('Audio 1');\n"
        ));
        if item.volume_db.abs() > 0.001 {
            script.push_str(&format!(
                "        trk_{idx}.mixerGroup.volume = {:.3};\n",
                item.volume_db
            ));
        }
        script.push_str(&format!(
            "        var clipLen_{idx} = asset_{idx}.length > 0 ? asset_{idx}.length : {:.4};\n",
            if item.duration > 0.0 { item.duration } else { 1.0 }
        ));
        script.push_str(&format!(
            "        var snd_{idx} = trk_{idx}.addSound(ev_{idx}.timeline, 'SingleSound', 0, clipLen_{idx});\n"
        ));
        script.push_str(&format!("        snd_{idx}.audioFile = asset_{idx};\n"));

        if item.looping {
            script.push_str(&format!(
                "        ev_{idx}.automatableProperties.isPersistent = true;\n"
            ));
            script.push_str(&format!(
                "        var mTrk_{idx} = ev_{idx}.markerTracks[0] || ev_{idx}.addMarkerTrack();\n"
            ));
            let start = item.loop_start.max(0.0);
            if item.custom_loop_enabled {
                if let Some(end) = item.loop_end {
                    let valid_end = if end > start { end } else { start + 0.1 };
                    let loop_len = valid_end - start;
                    script.push_str(&format!(
                        "        var lStart_{idx} = Math.min({:.4}, Math.max(0, clipLen_{idx} - 0.01));\n",
                        start
                    ));
                    script.push_str(&format!(
                        "        var lLen_{idx} = Math.max(0.01, Math.min({:.4}, clipLen_{idx} - lStart_{idx}));\n",
                        loop_len
                    ));
                    script.push_str(&format!(
                        "        mTrk_{idx}.addRegion(lStart_{idx}, lLen_{idx}, 'Loop', studio.project.regionLoopMode.Looping);\n"
                    ));
                } else {
                    script.push_str(&format!(
                        "        var lStart_{idx} = Math.min({:.4}, Math.max(0, clipLen_{idx} - 0.01));\n",
                        start
                    ));
                    script.push_str(&format!(
                        "        var lLen_{idx} = Math.max(0.01, clipLen_{idx} - lStart_{idx});\n"
                    ));
                    script.push_str(&format!(
                        "        mTrk_{idx}.addRegion(lStart_{idx}, lLen_{idx}, 'Loop', studio.project.regionLoopMode.Looping);\n"
                    ));
                }
            } else {
                script.push_str(&format!(
                    "        mTrk_{idx}.addRegion(0, clipLen_{idx}, 'Loop', studio.project.regionLoopMode.Looping);\n"
                ));
            }
        }

        script.push_str(&format!(
            "        ev_{idx}.relationships.banks.add(targetBank);\n"
        ));
        script.push_str("    }\n\n");
    }

    script.push_str("    var saved = studio.project.save();\n");
    script.push_str("    console.log('[BankBake] Project saved: ' + saved);\n");
    script.push_str("    var built = studio.project.build();\n");
    script.push_str("    console.log('[BankBake] Project built: ' + built);\n");
    if export_guids {
        script.push_str("    var exported = studio.project.exportGUIDs();\n");
        script.push_str("    console.log('[BankBake] GUIDs exported: ' + exported);\n");
    }
    script.push_str("})();\n");

    script
}

/// Execute the full bank compilation workflow
pub fn execute_bake_bank(request: &BakeBankRequest, config: &AppConfig) -> Result<BakeBankResult, String> {
    let start_instant = std::time::Instant::now();

    let bank_name = sanitize_bank_name(&request.bank_name)?;
    let raw_master = request
        .master_bank_name
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("Master");
    let master_bank_name = sanitize_bank_name(raw_master)?;
    if request.events.is_empty() {
        return Err("Please add at least one audio event before baking a bank.".to_string());
    }

    // Validate each event
    let mut seen_names = std::collections::HashSet::new();
    for ev in &request.events {
        let (folders, leaf_name) = parse_event_hierarchy(&ev.event_name, &ev.category)
            .map_err(|e| format!("Invalid event path for '{}': {}", ev.filename, e))?;
        let canonical_path = format!(
            "event:/{}/{}",
            folders.join("/").to_lowercase(),
            leaf_name.to_lowercase()
        );
        let p = PathBuf::from(&ev.file_path);
        if !p.exists() || !p.is_file() {
            return Err(format!("Source audio file not found: {}", ev.file_path));
        }
        if !seen_names.insert(canonical_path.clone()) {
            return Err(format!(
                "Duplicate event path 'event:/{}/{}'. Each event must have a unique path.",
                folders.join("/"),
                leaf_name
            ));
        }
        if ev.looping && ev.custom_loop_enabled {
            if ev.loop_start < 0.0 {
                return Err(format!("Loop start cannot be negative for event '{}'.", ev.event_name));
            }
            if let Some(end) = ev.loop_end {
                if end <= ev.loop_start {
                    return Err(format!(
                        "Loop end ({:.2}s) must be greater than loop start ({:.2}s) for event '{}'.",
                        end, ev.loop_start, ev.event_name
                    ));
                }
            }
        }
    }

    // Use the requested compiler path, falling back to the saved settings and automatic discovery.
    let compiler_override = request
        .fmod_compiler_path
        .as_deref()
        .or(config.fmod_compiler_path.as_deref());
    let compiler_info = discover_compiler(compiler_override);
    let compiler_exe = compiler_info.path.ok_or(compiler_info.message)?;

    // Determine output directory
    let out_dir_str = request
        .output_directory
        .as_deref()
        .or(config.output_directory.as_deref())
        .filter(|s| !s.trim().is_empty());

    let output_dir = if let Some(d) = out_dir_str {
        PathBuf::from(d)
    } else if let Ok(cwd) = std::env::current_dir() {
        let root = if cwd.ends_with("src-tauri") {
            cwd.parent().unwrap_or(&cwd).to_path_buf()
        } else {
            cwd
        };
        root.join("BakedBanks")
    } else {
        PathBuf::from("BakedBanks")
    };

    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output directory {}: {}", output_dir.display(), e))?;

    // Create isolated temporary build workspace
    let build_id = Uuid::new_v4().to_string();
    let temp_root = std::env::temp_dir()
        .join("BankBakeBuilds")
        .join(&build_id);

    // Fallback to workspace local temp directory if system temp dir is not writable
    let temp_workspace = if fs::create_dir_all(&temp_root).is_ok() {
        temp_root
    } else {
        let fallback = output_dir.join(".bankbake_tmp").join(&build_id);
        fs::create_dir_all(&fallback)
            .map_err(|e| format!("Failed to create temp build directory: {}", e))?;
        fallback
    };

    let project_dir = temp_workspace.join("Project");
    let local_appdata = temp_workspace.join("LocalAppData");
    let roaming_appdata = temp_workspace.join("AppData");
    let temp_env_dir = temp_workspace.join("Temp");
    let _ = fs::create_dir_all(&local_appdata);
    let _ = fs::create_dir_all(&roaming_appdata);
    let _ = fs::create_dir_all(&temp_env_dir);

    let fspro_path = create_temp_fmod_project(&project_dir, &master_bank_name)?;
    let build_master = request.build_master_banks.unwrap_or(config.build_master_banks);
    let export_guids = request.export_guids.unwrap_or(config.export_guids);
    let export_project = request.export_project_file.unwrap_or(config.export_project_file);
    let keep_temp = cfg!(debug_assertions) && config.keep_temp_project;

    #[cfg(target_os = "linux")]
    let wine_workspace = if compiler_info.source == "wine" {
        match wine_workspace_path(&temp_workspace) {
            Ok(path) => Some(path),
            Err(error) => {
                if !keep_temp {
                    let _ = fs::remove_dir_all(&temp_workspace);
                }
                return Err(error);
            }
        }
    } else {
        None
    };

    // Stage audio files into a clean folder with safe ASCII filenames (`audio_0.ogg`, etc.)
    // so that special characters, spaces, apostrophes, Unicode, or duplicate filenames
    // in the user's original paths never trip up FMOD Studio's JS engine or asset importer.
    let staged_dir = temp_workspace.join("StagedAudio");
    fs::create_dir_all(&staged_dir)
        .map_err(|e| format!("Failed to create staged audio directory: {}", e))?;

    let mut staged_events = request.events.clone();
    for (idx, ev) in staged_events.iter_mut().enumerate() {
        let src_path = PathBuf::from(&ev.file_path);
        let ext = src_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("wav")
            .to_lowercase();
        let safe_filename = format!("audio_{}.{}", idx, ext);
        let dst_path = staged_dir.join(&safe_filename);
        fs::copy(&src_path, &dst_path).map_err(|e| {
            format!(
                "Failed to stage audio file '{}' for compilation: {}",
                ev.filename, e
            )
        })?;
        ev.file_path = dst_path.to_string_lossy().to_string();
        #[cfg(target_os = "linux")]
        if let Some(workspace) = &wine_workspace {
            ev.file_path = format!("{workspace}/StagedAudio/{safe_filename}");
        }
    }

    let js_code = generate_fmod_bake_script(&bank_name, &master_bank_name, &staged_events, export_guids);
    let script_path = project_dir.join("bake.js");
    fs::write(&script_path, &js_code)
        .map_err(|e| format!("Failed to write bake.js script: {}", e))?;

    let mut command = Command::new(&compiler_exe);
    command.arg("-script").arg(&script_path).arg(&fspro_path);
    #[cfg(not(target_os = "linux"))]
    command
        .env("LOCALAPPDATA", &local_appdata)
        .env("APPDATA", &roaming_appdata)
        .env("TEMP", &temp_env_dir)
        .env("TMP", &temp_env_dir);
    #[cfg(target_os = "linux")]
    if let Some(workspace) = &wine_workspace {
        let fspro_filename = fspro_path.file_name().expect("FMOD project filename");
        command = Command::new("wine");
        command
            .arg(&compiler_exe)
            .arg("-script")
            .arg(format!("{workspace}/Project/bake.js"))
            .arg(format!("{workspace}/Project/{}", fspro_filename.to_string_lossy()))
            .env("LOCALAPPDATA", format!("{workspace}/LocalAppData"))
            .env("APPDATA", format!("{workspace}/AppData"))
            .env("TEMP", format!("{workspace}/Temp"))
            .env("TMP", format!("{workspace}/Temp"));
    } else {
        command
            .env("XDG_CONFIG_HOME", &roaming_appdata)
            .env("TMPDIR", &temp_env_dir);
    }

    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            if !keep_temp {
                let _ = fs::remove_dir_all(&temp_workspace);
            }
            return Err(format!(
                "Failed to execute FMOD compiler ({compiler_exe}): {error}"
            ));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let built_desktop_dir = project_dir.join("Build").join("Desktop");
    let target_bank_file = built_desktop_dir.join(format!("{}.bank", bank_name));

    if !target_bank_file.exists() {
        if !keep_temp {
            let _ = fs::remove_dir_all(&temp_workspace);
        }
        return Err(format!(
            "FMOD compiler finished without producing '{}.bank'.\nCompiler output:\n{}\n{}",
            bank_name, stdout, stderr
        ));
    }

    let mut built_files: Vec<BuiltFileEntry> = Vec::new();

    // Copy target bank
    let dest_bank = output_dir.join(format!("{}.bank", bank_name));
    fs::copy(&target_bank_file, &dest_bank)
        .map_err(|e| format!("Failed to copy built bank to output directory: {}", e))?;
    if let Ok(m) = fs::metadata(&dest_bank) {
        built_files.push(BuiltFileEntry {
            filename: format!("{}.bank", bank_name),
            path: dest_bank.to_string_lossy().to_string(),
            size_bytes: m.len(),
        });
    }

    // Copy <MasterBankName>.bank and <MasterBankName>.strings.bank if requested and not already the target bank
    if build_master {
        let master_files = [
            format!("{}.bank", master_bank_name),
            format!("{}.strings.bank", master_bank_name),
        ];
        for master_name in master_files {
            if master_name.eq_ignore_ascii_case(&format!("{}.bank", bank_name)) {
                continue;
            }
            let src = built_desktop_dir.join(&master_name);
            if src.exists() {
                let dst = output_dir.join(&master_name);
                if fs::copy(&src, &dst).is_ok() {
                    if let Ok(m) = fs::metadata(&dst) {
                        built_files.push(BuiltFileEntry {
                            filename: master_name,
                            path: dst.to_string_lossy().to_string(),
                            size_bytes: m.len(),
                        });
                    }
                }
            }
        }
    }

    // Copy GUIDs.txt if exported
    let mut guids_content = None;
    if export_guids {
        let guids_src = project_dir.join("Build").join("GUIDs.txt");
        if guids_src.exists() {
            if let Ok(txt) = fs::read_to_string(&guids_src) {
                guids_content = Some(txt);
            }
            let guids_dst = output_dir.join("GUIDs.txt");
            if fs::copy(&guids_src, &guids_dst).is_ok() {
                if let Ok(m) = fs::metadata(&guids_dst) {
                    built_files.push(BuiltFileEntry {
                        filename: "GUIDs.txt".to_string(),
                        path: guids_dst.to_string_lossy().to_string(),
                        size_bytes: m.len(),
                    });
                }
            }
        }
    }

    // Export .bankbake project archive (ZIP containing project.json + all audio files) if enabled
    if export_project {
        let project_filename = format!("{}.bankbake", bank_name);
        let project_dst = output_dir.join(&project_filename);
        if let Ok(project_size) = save_bankbake_project(
            &bank_name,
            Some(&master_bank_name),
            &request.events,
            &project_dst,
        ) {
            built_files.push(BuiltFileEntry {
                filename: project_filename,
                path: project_dst.to_string_lossy().to_string(),
                size_bytes: project_size,
            });
        }
    }

    if !keep_temp {
        let _ = fs::remove_dir_all(&temp_workspace);
    }

    let duration_ms = start_instant.elapsed().as_millis() as u64;

    Ok(BakeBankResult {
        success: true,
        bank_name: bank_name.clone(),
        output_directory: output_dir.to_string_lossy().to_string(),
        built_files,
        guids_content,
        compiler_stdout: stdout,
        compiler_stderr: stderr,
        generated_script: js_code,
        duration_ms,
        message: format!(
            "Successfully baked {}.bank ({} event{}) in {:.2}s",
            bank_name,
            request.events.len(),
            if request.events.len() == 1 { "" } else { "s" },
            duration_ms as f64 / 1000.0
        ),
    })
}

struct ZipWriteEntry {
    name: String,
    crc32: u32,
    method: u16,
    compressed_size: u32,
    uncompressed_size: u32,
    local_header_offset: u32,
}

fn sanitize_archive_filename(name: &str) -> String {
    let forbidden = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if forbidden.contains(&c) || c.is_control() {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() {
        "audio.wav".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Write a standard ZIP archive containing the given `(entry_name, raw_bytes, compress)` files
fn write_zip_archive(dest_path: &Path, files: &[(String, Vec<u8>, bool)]) -> Result<u64, String> {
    if let Some(parent) = dest_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
    }

    let mut buf: Vec<u8> = Vec::new();
    let mut central_entries: Vec<ZipWriteEntry> = Vec::with_capacity(files.len());

    for (name, raw_data, should_compress) in files {
        let local_header_offset = buf.len() as u32;
        let crc = crc32fast::hash(raw_data);
        let uncompressed_size = raw_data.len() as u32;

        let (method, payload): (u16, Vec<u8>) = if *should_compress && !raw_data.is_empty() {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            if encoder.write_all(raw_data).is_ok() {
                if let Ok(compressed) = encoder.finish() {
                    if compressed.len() < raw_data.len() {
                        (8, compressed)
                    } else {
                        (0, raw_data.clone())
                    }
                } else {
                    (0, raw_data.clone())
                }
            } else {
                (0, raw_data.clone())
            }
        } else {
            (0, raw_data.clone())
        };

        let compressed_size = payload.len() as u32;
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() as u16;
        // Bit 11 (0x0800) = UTF-8 filename encoding
        let flags: u16 = 0x0800;

        // Local file header (30 bytes + filename)
        buf.extend_from_slice(&0x04034b50u32.to_le_bytes());
        buf.extend_from_slice(&20u16.to_le_bytes()); // version needed to extract (2.0)
        buf.extend_from_slice(&flags.to_le_bytes());
        buf.extend_from_slice(&method.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // mod time
        buf.extend_from_slice(&0u16.to_le_bytes()); // mod date
        buf.extend_from_slice(&crc.to_le_bytes());
        buf.extend_from_slice(&compressed_size.to_le_bytes());
        buf.extend_from_slice(&uncompressed_size.to_le_bytes());
        buf.extend_from_slice(&name_len.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // extra field len
        buf.extend_from_slice(name_bytes);
        buf.extend_from_slice(&payload);

        central_entries.push(ZipWriteEntry {
            name: name.clone(),
            crc32: crc,
            method,
            compressed_size,
            uncompressed_size,
            local_header_offset,
        });
    }

    let central_dir_offset = buf.len() as u32;
    for entry in &central_entries {
        let name_bytes = entry.name.as_bytes();
        let name_len = name_bytes.len() as u16;
        let flags: u16 = 0x0800;

        // Central directory file header (46 bytes + filename)
        buf.extend_from_slice(&0x02014b50u32.to_le_bytes());
        buf.extend_from_slice(&20u16.to_le_bytes()); // version made by
        buf.extend_from_slice(&20u16.to_le_bytes()); // version needed
        buf.extend_from_slice(&flags.to_le_bytes());
        buf.extend_from_slice(&entry.method.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // mod time
        buf.extend_from_slice(&0u16.to_le_bytes()); // mod date
        buf.extend_from_slice(&entry.crc32.to_le_bytes());
        buf.extend_from_slice(&entry.compressed_size.to_le_bytes());
        buf.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
        buf.extend_from_slice(&name_len.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // extra len
        buf.extend_from_slice(&0u16.to_le_bytes()); // comment len
        buf.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        buf.extend_from_slice(&0u16.to_le_bytes()); // internal file attrs
        buf.extend_from_slice(&0u32.to_le_bytes()); // external file attrs
        buf.extend_from_slice(&entry.local_header_offset.to_le_bytes());
        buf.extend_from_slice(name_bytes);
    }

    let central_dir_size = (buf.len() as u32) - central_dir_offset;
    let entry_count = central_entries.len() as u16;

    // End of Central Directory Record (22 bytes)
    buf.extend_from_slice(&0x06054b50u32.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes()); // number of this disk
    buf.extend_from_slice(&0u16.to_le_bytes()); // disk where central directory starts
    buf.extend_from_slice(&entry_count.to_le_bytes());
    buf.extend_from_slice(&entry_count.to_le_bytes());
    buf.extend_from_slice(&central_dir_size.to_le_bytes());
    buf.extend_from_slice(&central_dir_offset.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes()); // comment length

    fs::write(dest_path, &buf)
        .map_err(|e| format!("Failed to write .bankbake file {}: {}", dest_path.display(), e))?;
    Ok(buf.len() as u64)
}

/// Read all entries from a standard ZIP archive (supporting Stored method 0 and Deflate method 8)
fn read_zip_archive(zip_path: &Path) -> Result<HashMap<String, Vec<u8>>, String> {
    let bytes = fs::read(zip_path)
        .map_err(|e| format!("Failed to read .bankbake file {}: {}", zip_path.display(), e))?;

    if bytes.len() < 22 {
        return Err("Invalid .bankbake project file (file is too small to be a ZIP archive).".to_string());
    }

    // Locate End of Central Directory (EOCD) signature 0x06054b50 scanning backwards from end
    let min_eocd_pos = bytes.len().saturating_sub(65557);
    let mut eocd_pos: Option<usize> = None;
    for i in (min_eocd_pos..=bytes.len() - 22).rev() {
        if &bytes[i..i + 4] == &[0x50, 0x4b, 0x05, 0x06] {
            eocd_pos = Some(i);
            break;
        }
    }

    let eocd = eocd_pos.ok_or_else(|| {
        "Invalid .bankbake project file (End of Central Directory not found).".to_string()
    })?;

    let entry_count = u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]) as usize;
    let cd_offset = u32::from_le_bytes([
        bytes[eocd + 16],
        bytes[eocd + 17],
        bytes[eocd + 18],
        bytes[eocd + 19],
    ]) as usize;

    let mut entries = HashMap::with_capacity(entry_count);
    let mut pos = cd_offset;

    for _ in 0..entry_count {
        if pos + 46 > bytes.len() || &bytes[pos..pos + 4] != &[0x50, 0x4b, 0x01, 0x02] {
            return Err("Corrupted Central Directory in .bankbake project archive.".to_string());
        }
        let method = u16::from_le_bytes([bytes[pos + 10], bytes[pos + 11]]);
        let compressed_size = u32::from_le_bytes([
            bytes[pos + 20],
            bytes[pos + 21],
            bytes[pos + 22],
            bytes[pos + 23],
        ]) as usize;
        let uncompressed_size = u32::from_le_bytes([
            bytes[pos + 24],
            bytes[pos + 25],
            bytes[pos + 26],
            bytes[pos + 27],
        ]) as usize;
        let name_len = u16::from_le_bytes([bytes[pos + 28], bytes[pos + 29]]) as usize;
        let extra_len = u16::from_le_bytes([bytes[pos + 30], bytes[pos + 31]]) as usize;
        let comment_len = u16::from_le_bytes([bytes[pos + 32], bytes[pos + 33]]) as usize;
        let local_header_offset = u32::from_le_bytes([
            bytes[pos + 42],
            bytes[pos + 43],
            bytes[pos + 44],
            bytes[pos + 45],
        ]) as usize;

        if pos + 46 + name_len > bytes.len() {
            return Err("Corrupted entry filename in .bankbake project archive.".to_string());
        }
        let name = String::from_utf8_lossy(&bytes[pos + 46..pos + 46 + name_len])
            .replace('\\', "/");

        pos += 46 + name_len + extra_len + comment_len;

        // Skip directory entries
        if name.ends_with('/') {
            continue;
        }

        if local_header_offset + 30 > bytes.len()
            || &bytes[local_header_offset..local_header_offset + 4] != &[0x50, 0x4b, 0x03, 0x04]
        {
            return Err(format!(
                "Corrupted Local File Header for '{}' in .bankbake archive.",
                name
            ));
        }

        let local_name_len = u16::from_le_bytes([
            bytes[local_header_offset + 26],
            bytes[local_header_offset + 27],
        ]) as usize;
        let local_extra_len = u16::from_le_bytes([
            bytes[local_header_offset + 28],
            bytes[local_header_offset + 29],
        ]) as usize;

        let data_start = local_header_offset + 30 + local_name_len + local_extra_len;
        let data_end = data_start + compressed_size;
        if data_end > bytes.len() {
            return Err(format!(
                "Truncated data stream for '{}' in .bankbake archive.",
                name
            ));
        }

        let raw_slice = &bytes[data_start..data_end];
        let decoded = match method {
            0 => raw_slice.to_vec(),
            8 => {
                let mut decoder = DeflateDecoder::new(raw_slice);
                let mut out = Vec::with_capacity(uncompressed_size);
                decoder.read_to_end(&mut out).map_err(|e| {
                    format!("Failed to decompress '{}' from .bankbake archive: {}", name, e)
                })?;
                out
            }
            other => {
                return Err(format!(
                    "Unsupported ZIP compression method {} for '{}' in .bankbake archive.",
                    other, name
                ));
            }
        };

        entries.insert(name, decoded);
    }

    Ok(entries)
}

/// Save a `.bankbake` project file (ZIP archive containing `project.json` and all audio files under `assets/`)
pub fn save_bankbake_project(
    bank_name: &str,
    master_bank_name: Option<&str>,
    events: &[BakeEventItem],
    dest_path: &Path,
) -> Result<u64, String> {
    let clean_bank_name = sanitize_bank_name(bank_name)?;
    let clean_master_name = match master_bank_name.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(m) => sanitize_bank_name(m)?,
        None => "Master".to_string(),
    };
    let mut project_events: Vec<ProjectEventEntry> = Vec::with_capacity(events.len());
    let mut zip_files: Vec<(String, Vec<u8>, bool)> = Vec::with_capacity(events.len() + 1);

    for (idx, ev) in events.iter().enumerate() {
        let src_path = PathBuf::from(&ev.file_path);
        if !src_path.exists() || !src_path.is_file() {
            return Err(format!("Audio file not found: {}", ev.file_path));
        }
        let audio_bytes = fs::read(&src_path).map_err(|e| {
            format!(
                "Failed to read audio file '{}' for project archive: {}",
                ev.filename, e
            )
        })?;

        let raw_filename = if !ev.filename.trim().is_empty() {
            ev.filename.clone()
        } else {
            src_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("audio.wav")
                .to_string()
        };
        let safe_name = sanitize_archive_filename(&raw_filename);
        let archive_path = format!("assets/{}_{}", idx, safe_name);

        let ext = src_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        // Compress uncompressed PCM formats (wav, aif, aiff), store already-compressed formats (ogg, mp3, flac, wma)
        let should_compress = matches!(ext.as_str(), "wav" | "aif" | "aiff");

        project_events.push(ProjectEventEntry {
            id: ev.id.clone(),
            archive_path: archive_path.clone(),
            filename: raw_filename,
            event_name: ev.event_name.clone(),
            category: ev.category.clone(),
            looping: ev.looping,
            volume_db: ev.volume_db,
            custom_loop_enabled: ev.custom_loop_enabled,
            loop_start: ev.loop_start,
            loop_end: ev.loop_end,
            duration: ev.duration,
        });

        zip_files.push((archive_path, audio_bytes, should_compress));
    }

    let manifest = BankBakeProjectManifest {
        format: "bankbake-project".to_string(),
        version: "1.1.0".to_string(),
        bank_name: clean_bank_name,
        master_bank_name: Some(clean_master_name),
        events: project_events,
    };

    let manifest_json = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize project.json: {}", e))?;

    // Put project.json first in the ZIP archive
    let mut ordered_files = Vec::with_capacity(zip_files.len() + 1);
    ordered_files.push(("project.json".to_string(), manifest_json, true));
    ordered_files.extend(zip_files);

    write_zip_archive(dest_path, &ordered_files)
}

/// Load a `.bankbake` project file, extracting embedded audio files to a session workspace directory
pub fn load_bankbake_project(project_path: &Path) -> Result<LoadedProjectData, String> {
    if !project_path.exists() || !project_path.is_file() {
        return Err(format!(
            "Project file does not exist: {}",
            project_path.display()
        ));
    }

    let mut entries = read_zip_archive(project_path)?;
    let manifest_bytes = entries.remove("project.json").ok_or_else(|| {
        "Invalid .bankbake file: missing 'project.json' manifest inside archive.".to_string()
    })?;

    let manifest: BankBakeProjectManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("Failed to parse 'project.json' inside .bankbake archive: {}", e))?;

    let session_id = Uuid::new_v4().to_string();
    let primary_extract_dir = std::env::temp_dir()
        .join("BankBakeProjects")
        .join(&session_id);

    let extract_dir = if fs::create_dir_all(&primary_extract_dir).is_ok() {
        primary_extract_dir
    } else {
        let fallback = project_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".bankbake_cache")
            .join(&session_id);
        fs::create_dir_all(&fallback)
            .map_err(|e| format!("Failed to create project extraction cache directory: {}", e))?;
        fallback
    };

    let mut loaded_events = Vec::with_capacity(manifest.events.len());

    for (idx, ev) in manifest.events.into_iter().enumerate() {
        let normalized_key = ev.archive_path.replace('\\', "/");
        let audio_bytes = entries.remove(&normalized_key).ok_or_else(|| {
            format!(
                "Missing embedded audio asset '{}' in .bankbake project file.",
                ev.archive_path
            )
        })?;

        let safe_name = sanitize_archive_filename(&ev.filename);
        let extracted_file_path = extract_dir.join(format!("{}_{}", idx, safe_name));
        fs::write(&extracted_file_path, &audio_bytes).map_err(|e| {
            format!(
                "Failed to extract audio asset '{}' from project file: {}",
                ev.filename, e
            )
        })?;

        let size_bytes = audio_bytes.len() as u64;
        let duration = if ev.duration > 0.0 {
            ev.duration
        } else {
            probe_audio_duration(&extracted_file_path).unwrap_or(0.0)
        };

        loaded_events.push(LoadedProjectEvent {
            id: if ev.id.trim().is_empty() {
                Uuid::new_v4().to_string()
            } else {
                ev.id
            },
            file_path: extracted_file_path.to_string_lossy().to_string(),
            filename: ev.filename,
            event_name: ev.event_name,
            category: ev.category,
            looping: ev.looping,
            volume_db: ev.volume_db,
            custom_loop_enabled: ev.custom_loop_enabled,
            loop_start: ev.loop_start,
            loop_end: ev.loop_end,
            duration,
            size_bytes,
        });
    }

    let master_bank_name = manifest
        .master_bank_name
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("Master")
        .to_string();

    Ok(LoadedProjectData {
        bank_name: manifest.bank_name,
        master_bank_name,
        project_path: project_path.to_string_lossy().to_string(),
        events: loaded_events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_discovers_extensionless_compiler_in_configured_directory() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("bankbake_compiler_{}", Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let cli = dir.join("fmodstudiocl");
        fs::write(&cli, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o644)).unwrap();
        let before = discover_compiler(dir.to_str());
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o755)).unwrap();
        let after = discover_compiler(dir.to_str());
        fs::remove_dir_all(&dir).unwrap();

        assert_ne!(before.path.as_deref(), cli.to_str());
        assert!(after.available);
        assert_eq!(after.source, "configured");
        assert_eq!(after.path.as_deref(), cli.to_str());
    }

    #[test]
    #[cfg_attr(target_os = "linux", ignore = "requires FMOD Studio or Wine on Linux")]
    fn test_discover_compiler_and_bake_bank() {
        let info = discover_compiler(None);
        assert!(info.available, "{}", info.message);

        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let sample_ogg = workspace_root.join("samples").join("ui_cancel.ogg");
        assert!(sample_ogg.exists(), "Sample OGG asset must exist");

        let inspected = inspect_audio_file(&sample_ogg.to_string_lossy()).expect("inspect audio");
        assert_eq!(inspected.suggested_event_name, "ui_cancel");
        assert!(inspected.duration_seconds.unwrap_or(0.0) > 0.5);

        let out_dir = std::env::temp_dir().join(format!("bankbake_test_{}", Uuid::new_v4()));

        let req = BakeBankRequest {
            bank_name: "RustTestBank".to_string(),
            master_bank_name: Some("Master".to_string()),
            events: vec![
                BakeEventItem {
                    id: "1".to_string(),
                    file_path: sample_ogg.to_string_lossy().to_string(),
                    filename: "ui_cancel.ogg".to_string(),
                    event_name: "event:/sfx/player/ui_cancel_sfx".to_string(),
                    category: EventCategory::Sfx,
                    looping: false,
                    volume_db: -1.5,
                    custom_loop_enabled: false,
                    loop_start: 0.0,
                    loop_end: None,
                    duration: 1.026,
                },
                BakeEventItem {
                    id: "2".to_string(),
                    file_path: sample_ogg.to_string_lossy().to_string(),
                    filename: "ui_cancel.ogg".to_string(),
                    event_name: "event:/music/menu_loop_bgm".to_string(),
                    category: EventCategory::Music,
                    looping: true,
                    volume_db: -3.0,
                    custom_loop_enabled: true,
                    loop_start: 0.1,
                    loop_end: Some(0.9),
                    duration: 1.026,
                },
            ],
            mode: Some(BakeMode::Standalone),
            output_directory: Some(out_dir.to_string_lossy().to_string()),
            fmod_compiler_path: info.path.clone(),
            build_master_banks: Some(true),
            export_guids: Some(true),
            export_project_file: Some(true),
        };

        let mut cfg = AppConfig::default();
        cfg.output_directory = Some(out_dir.to_string_lossy().to_string());

        let result = execute_bake_bank(&req, &cfg).expect("Bake bank should succeed");
        assert!(result.success);
        assert!(out_dir.join("RustTestBank.bank").exists());
        assert!(out_dir.join("Master.bank").exists());
        assert!(out_dir.join("Master.strings.bank").exists());
        assert!(out_dir.join("GUIDs.txt").exists());
        let project_file = out_dir.join("RustTestBank.bankbake");
        assert!(project_file.exists(), "Expected RustTestBank.bankbake project file to be generated");

        let loaded_project = load_bankbake_project(&project_file)
            .expect("Expected generated .bankbake project file to load cleanly");
        assert_eq!(loaded_project.bank_name, "RustTestBank");
        assert_eq!(loaded_project.events.len(), 2);
        assert_eq!(loaded_project.events[0].event_name, "event:/sfx/player/ui_cancel_sfx");
        assert_eq!(loaded_project.events[1].event_name, "event:/music/menu_loop_bgm");
        assert!(loaded_project.events[1].looping);
        assert!(loaded_project.events[1].custom_loop_enabled);
        assert!(PathBuf::from(&loaded_project.events[0].file_path).exists());

        let guids = result.guids_content.expect("GUIDs content present");
        assert!(guids.contains("event:/sfx/player/ui_cancel_sfx"));
        assert!(guids.contains("event:/music/menu_loop_bgm"));

        // Verify a second bake produces a distinct Master.bank / Master.strings.bank GUID
        // so multiple mods can load their Master.bank and Master.strings.bank concurrently.
        let out_dir_2 = out_dir.join("second_bake");
        let mut req2 = req.clone();
        req2.bank_name = "SecondModBank".to_string();
        req2.master_bank_name = Some("CustomMaster".to_string());
        req2.output_directory = Some(out_dir_2.to_string_lossy().to_string());
        let result2 = execute_bake_bank(&req2, &cfg).expect("Second bake should succeed");
        assert!(out_dir_2.join("CustomMaster.bank").exists());
        assert!(out_dir_2.join("CustomMaster.strings.bank").exists());
        let guids2 = result2.guids_content.expect("Second GUIDs content present");

        let master_guid_1 = guids
            .lines()
            .find(|l| l.ends_with(" bank:/Master"))
            .and_then(|l| l.split_whitespace().next())
            .expect("bank:/Master in guids 1");
        let master_guid_2 = guids2
            .lines()
            .find(|l| l.ends_with(" bank:/CustomMaster"))
            .and_then(|l| l.split_whitespace().next())
            .expect("bank:/CustomMaster in guids 2");
        assert_ne!(
            master_guid_1, master_guid_2,
            "Each baked mod must get a unique Master bank GUID so multiple mods don't collide"
        );

        let _ = fs::remove_dir_all(&out_dir);
    }
}
