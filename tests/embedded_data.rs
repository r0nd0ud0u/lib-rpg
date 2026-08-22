//! Verifies that `utils::set_embedded_files` makes `DataManager::try_new` (and the
//! individual loaders it calls) produce the exact same result as the real-filesystem
//! path, by walking the real `tests/offlines` fixture into an in-memory map and feeding
//! that to `set_embedded_files` instead of leaving the filesystem to be read directly.
//!
//! Lives under `tests/` (a separate binary from `src/`'s unit tests) deliberately:
//! `EMBEDDED_FILES` is a process-global `OnceLock` — first caller wins, for the process's
//! whole lifetime — so calling `set_embedded_files` inside the main unit-test binary would
//! permanently switch every other test (which expects real-fs behavior) onto the embedded
//! path too, depending on test execution order/interleaving.

use std::{collections::HashMap, fs, path::Path};

use lib_rpg::{server::data_manager::DataManager, utils::set_embedded_files};

/// Recursively walks `root`, returning every file's path (exactly as `Path::join` would
/// produce it from `root`) mapped to its content, leaked to `'static` — matching the shape
/// `set_embedded_files` expects (mirroring how compile-time `include_str!` embedding would
/// hand out `&'static str` content in the real build.rs-generated version).
fn walk_to_embedded_map(root: &Path) -> HashMap<String, &'static str> {
    let mut map = HashMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read_dir({dir:?}): {e}")) {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let content = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("read_to_string({path:?}): {e}"));
                let key = path.to_string_lossy().replace('\\', "/");
                map.insert(key, Box::leak(content.into_boxed_str()) as &'static str);
            }
        }
    }
    map
}

// A single test function, deliberately: `set_embedded_files` is "first caller wins" for
// the whole process, so two tests each wanting a different embedded map would race —
// whichever runs first (test binaries run tests on parallel threads by default) would
// silently win for both, since the second `set_embedded_files` call becomes a no-op.
#[test]
fn data_manager_try_new_matches_real_fs_counts_via_embedded_files() {
    let root = Path::new("./tests/offlines");
    set_embedded_files(walk_to_embedded_map(root));

    // Same expectations `src/server/data_manager.rs`'s `unit_try_new`/
    // `unit_load_all_characters`/`unit_load_all_scenarios`/`unit_load_all_talent_trees`
    // assert against the real filesystem for this exact fixture data.
    let dm = DataManager::try_new(root).expect("DataManager::try_new over embedded files");
    assert_eq!(dm.all_heroes.len(), 2);
    assert_eq!(dm.all_bosses.len(), 2);
    assert_eq!(dm.all_scenarios.len(), 2);
    assert_eq!(dm.talent_trees.len(), 1);
    assert!(dm.talent_tree_for("test").is_some());
    assert_eq!(
        {
            use lib_rpg::character_mod::equipment::EquipmentJsonKey;
            use strum::IntoEnumIterator;
            EquipmentJsonKey::iter().count()
        },
        dm.equipment_table.len()
    );

    // A root path with no matching entries in the (already-populated) embedded map —
    // every loader should come back empty/Ok, not error, matching the "missing dir is
    // fine" real-fs behavior load_all_talent_trees documents.
    let mut empty = DataManager::default();
    assert!(
        empty
            .load_all_talent_trees("./tests/offlines/nonexistent_root")
            .is_ok()
    );
    assert!(empty.talent_trees.is_empty());
}
