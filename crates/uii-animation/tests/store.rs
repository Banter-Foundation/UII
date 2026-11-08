use uii_animation::{load_animation, save_animation, AnimationProject};

#[test]
fn clip_file_round_trips_and_a_bad_file_is_left_in_place() {
    let dir = std::env::temp_dir().join(format!("uii-animation-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("animation.json");
    let missing = load_animation(&path).unwrap();
    assert!(missing.project.clips().is_empty());
    assert!(missing.status.is_empty());

    let mut project = AnimationProject::new();
    let clip = project.add_clip("Intro");
    save_animation(&path, &project).unwrap();
    let loaded = load_animation(&path).unwrap();
    assert_eq!(loaded.project.clips().len(), 1);
    assert_eq!(loaded.project.clip(clip).unwrap().name(), "Intro");

    std::fs::write(&path, "{not json").unwrap();
    let bad = load_animation(&path).unwrap();
    assert!(bad.project.clips().is_empty());
    assert!(bad.status.contains("json.bad"), "{}", bad.status);
    assert!(path.with_extension("json.bad").is_file());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
    let _ = std::fs::remove_dir_all(&dir);
}
