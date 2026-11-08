//! Clip file for the Animation tab. The editor window is the host.
//! `UII_ANIMATION_PATH` overrides the default `animation.json`.

use std::path::{Path, PathBuf};

use crate::model::AnimationProject;

pub struct AnimationFile {
    pub project: AnimationProject,
    pub status: String,
}

pub fn animation_path() -> PathBuf {
    if let Some(path) = std::env::var_os("UII_ANIMATION_PATH") {
        return PathBuf::from(path);
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("uii").join("animation.json")
}

/// A missing file is an empty project. A file that cannot be read is left in
/// place and copied to `animation.json.bad`.
pub fn load_animation(path: &Path) -> Result<AnimationFile, String> {
    match std::fs::read_to_string(path) {
        Ok(text) if !text.trim().is_empty() => match AnimationProject::from_json(&text) {
            Ok(project) => Ok(AnimationFile {
                project,
                status: String::new(),
            }),
            Err(_) => {
                let backup = path.with_extension("json.bad");
                let _ = std::fs::copy(path, &backup);
                Ok(AnimationFile {
                    project: AnimationProject::new(),
                    status: format!(
                        "Could not read {}. Left the file in place and copied it to {}.",
                        path.display(),
                        backup.display()
                    ),
                })
            }
        },
        Ok(_) => Ok(AnimationFile {
            project: AnimationProject::new(),
            status: String::new(),
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(AnimationFile {
            project: AnimationProject::new(),
            status: String::new(),
        }),
        Err(err) => Err(format!("read {}: {err}", path.display())),
    }
}

pub fn save_animation(path: &Path, project: &AnimationProject) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
    }
    std::fs::write(path, project.to_json())
        .map_err(|err| format!("write {}: {err}", path.display()))
}
