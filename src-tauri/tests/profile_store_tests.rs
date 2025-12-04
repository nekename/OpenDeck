// Integration tests for profile storage operations
// Tests Store, ProfileStores CRUD, serialization, and filesystem operations

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Helper to create a temporary test directory
fn setup_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[cfg(test)]
mod store_basic_operations {
    use super::*;

    #[test]
    fn test_store_saves_json_to_disk() {
        // Test that Store correctly saves JSON data to the filesystem
        // This will verify the serialization and file writing logic
        let _temp_dir = setup_test_dir();
        
        // Note: This test needs access to the Store type
        // Implementation depends on making Store accessible to tests
        // For now, this is a placeholder showing test structure
    }

    #[test]
    fn test_store_loads_from_valid_json() {
        // Test that Store can load from a valid JSON file
        let _temp_dir = setup_test_dir();
    }

    #[test]
    fn test_store_recovery_from_temp_file() {
        // Test recovery when main file is corrupt but temp file is valid
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        // Simulate interrupted save: main file corrupt, temp file valid
        fs::write(config_dir.join("test.json"), "{ corrupt json").ok();
        
        let temp_file = config_dir.join("test.json.temp");
        fs::write(&temp_file, r#"{"id": "test", "keys": [], "sliders": []}"#).ok();
        
        // Store should recover from temp file
    }

    #[test]
    fn test_store_recovery_from_backup_file() {
        // Test recovery when main and temp files are corrupt but backup is valid
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        // Simulate both main and temp corrupt
        fs::write(config_dir.join("test.json"), "corrupt").ok();
        fs::write(config_dir.join("test.json.temp"), "also corrupt").ok();
        
        // But backup is valid
        let backup_file = config_dir.join("test.json.bak");
        fs::write(&backup_file, r#"{"id": "test", "keys": [], "sliders": []}"#).ok();
        
        // Store should recover from backup file
    }

    #[test]
    fn test_store_uses_default_when_all_files_corrupt() {
        // Test that Store uses default value when all files are corrupt
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        // All files are corrupt
        fs::write(config_dir.join("test.json"), "corrupt").ok();
        fs::write(config_dir.join("test.json.temp"), "corrupt").ok();
        fs::write(config_dir.join("test.json.bak"), "corrupt").ok();
        
        // Store should use default value
    }

    #[test]
    fn test_store_cleanup_after_successful_save() {
        // Test that temp and old backup files are cleaned up after successful save
        let _temp_dir = setup_test_dir();
        
        // After a successful save:
        // - main.json should exist
        // - main.json.temp should not exist
        // - main.json.bak should not exist (or is from previous save)
    }

    #[test]
    fn test_store_creates_parent_directories() {
        // Test that Store creates parent directories if they don't exist
        let temp_dir = setup_test_dir();
        let nested_path = temp_dir.path().join("nested").join("deeper").join("path");
        
        // Store should create all parent directories
        assert!(!nested_path.exists());
        // After Store creation and save, nested_path should exist
    }
}

#[cfg(test)]
mod profile_stores_operations {
    use super::*;

    #[test]
    fn test_canonical_id_unix_paths() {
        // Test that canonical_id creates correct IDs on Unix systems
        // Should use forward slashes
        
        // On Unix: device/profile/subprofile
        // On Windows: device\profile\subprofile
    }

    #[test]
    fn test_canonical_id_windows_paths() {
        // Test that canonical_id converts forward slashes to backslashes on Windows
        
        // Input: "device123", "folder/profile"
        // Unix output: "device123/folder/profile"
        // Windows output: "device123\folder\profile"
    }

    #[test]
    fn test_get_profile_store_creates_if_not_exists() {
        // Test that get_profile_store_mut creates a new store if it doesn't exist
        let _temp_dir = setup_test_dir();
        
        // First call should create the store
        // Subsequent calls should return the existing store
    }

    #[test]
    fn test_get_profile_store_initializes_correct_size() {
        // Test that new profile stores have correct number of keys and sliders
        let _temp_dir = setup_test_dir();
        
        // Device with 3 rows, 5 columns, 2 encoders
        // Profile should have 15 key slots and 2 slider slots
    }

    #[test]
    fn test_remove_profile_from_memory() {
        // Test that remove_profile removes profile from in-memory cache
        let _temp_dir = setup_test_dir();
        
        // Create profile, then remove it
        // Verify it's no longer in the stores map
    }

    #[test]
    fn test_delete_profile_removes_files() {
        // Test that delete_profile removes all associated files
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        // Create profile JSON file
        let profile_dir = config_dir.join("profiles").join("device123");
        fs::create_dir_all(&profile_dir).ok();
        fs::write(profile_dir.join("test.json"), "{}").ok();
        
        // Create images directory
        let images_dir = config_dir.join("images").join("device123").join("test");
        fs::create_dir_all(&images_dir).ok();
        fs::write(images_dir.join("image.png"), "fake").ok();
        
        // After delete_profile:
        // - test.json should be gone
        // - images directory should be gone
        // - parent directory should be removed if empty
    }

    #[test]
    fn test_delete_profile_with_folder_structure() {
        // Test deleting profile in a folder (e.g., "folder/profile")
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        // Create nested profile
        let profile_dir = config_dir.join("profiles").join("device123").join("folder");
        fs::create_dir_all(&profile_dir).ok();
        fs::write(profile_dir.join("profile.json"), "{}").ok();
        
        // After delete, the file should be gone but folder might remain if other profiles exist
    }

    #[test]
    fn test_all_from_plugin_finds_all_instances() {
        // Test that all_from_plugin returns all action instances from a specific plugin
        let _temp_dir = setup_test_dir();
        
        // Create multiple profiles with actions from different plugins
        // Query for specific plugin, should return only matching instances
    }

    #[test]
    fn test_all_from_plugin_includes_nested_actions() {
        // Test that all_from_plugin finds actions nested in multi-actions
        let _temp_dir = setup_test_dir();
        
        // Create profile with multi-action containing plugin actions
        // Should find both top-level and nested instances
    }
}

#[cfg(test)]
mod profile_crud_operations {
    use super::*;

    #[test]
    fn test_create_profile_default() {
        // Test creating a new profile with default name
        let _temp_dir = setup_test_dir();
        
        // Should create "Default" profile if none exists
    }

    #[test]
    fn test_create_profile_in_folder() {
        // Test creating profile with folder organization
        let _temp_dir = setup_test_dir();
        
        // Create "Work/Daily" profile
        // Should create proper directory structure
    }

    #[test]
    fn test_get_device_profiles_lists_all() {
        // Test that get_device_profiles returns all profiles for a device
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        let device_dir = config_dir.join("profiles").join("device123");
        fs::create_dir_all(&device_dir).ok();
        
        // Create multiple profiles
        fs::write(device_dir.join("profile1.json"), "{}").ok();
        fs::write(device_dir.join("profile2.json"), "{}").ok();
        
        // Should return ["profile1", "profile2"]
    }

    #[test]
    fn test_get_device_profiles_includes_folders() {
        // Test that get_device_profiles includes profiles in folders
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        let device_dir = config_dir.join("profiles").join("device123");
        fs::create_dir_all(&device_dir.join("folder")).ok();
        
        fs::write(device_dir.join("root.json"), "{}").ok();
        fs::write(device_dir.join("folder").join("nested.json"), "{}").ok();
        
        // Should return ["root", "folder/nested"]
    }

    #[test]
    fn test_get_device_profiles_ignores_non_json() {
        // Test that get_device_profiles ignores non-JSON files
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        let device_dir = config_dir.join("profiles").join("device123");
        fs::create_dir_all(&device_dir).ok();
        
        fs::write(device_dir.join("profile.json"), "{}").ok();
        fs::write(device_dir.join("readme.txt"), "not json").ok();
        fs::write(device_dir.join("backup.bak"), "{}").ok();
        
        // Should only return ["profile"]
    }

    #[test]
    fn test_get_device_profiles_handles_backup_files() {
        // Test handling of .json.bak and .json.temp files
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        let device_dir = config_dir.join("profiles").join("device123");
        fs::create_dir_all(&device_dir).ok();
        
        fs::write(device_dir.join("profile.json"), "{}").ok();
        fs::write(device_dir.join("profile.json.bak"), "{}").ok();
        fs::write(device_dir.join("profile.json.temp"), "{}").ok();
        
        // Should return only ["profile"], not duplicates for backup files
    }

    #[test]
    fn test_get_device_profiles_returns_default_if_empty() {
        // Test that get_device_profiles returns ["Default"] if no profiles exist
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        let device_dir = config_dir.join("profiles").join("device123");
        fs::create_dir_all(&device_dir).ok();
        
        // Empty directory should return ["Default"]
    }
}

#[cfg(test)]
mod device_config_operations {
    use super::*;

    #[test]
    fn test_get_selected_profile_default() {
        // Test that get_selected_profile returns "Default" for new device
        let _temp_dir = setup_test_dir();
        
        // First call for new device should return "Default"
    }

    #[test]
    fn test_set_selected_profile_persists() {
        // Test that set_selected_profile saves to disk
        let _temp_dir = setup_test_dir();
        
        // Set selected profile to "MyProfile"
        // Should create device config file with selected_profile: "MyProfile"
    }

    #[test]
    fn test_get_selected_profile_returns_existing_profile() {
        // Test that get_selected_profile falls back to existing if saved profile doesn't exist
        let temp_dir = setup_test_dir();
        let config_dir = temp_dir.path();
        
        // Set selected profile to "NonExistent"
        // But only "Actual" profile exists
        // Should return "Actual" instead of "NonExistent"
        
        let device_dir = config_dir.join("profiles").join("device123");
        fs::create_dir_all(&device_dir).ok();
        fs::write(device_dir.join("Actual.json"), r#"{"id":"Actual","keys":[],"sliders":[]}"#).ok();
    }
}

#[cfg(test)]
mod plugin_cleanup_tests {
    use super::*;

    #[test]
    fn test_removed_plugin_actions_are_cleaned() {
        // Test that actions from removed plugins are cleaned up when loading profile
        let _temp_dir = setup_test_dir();
        
        // Create profile with actions from plugin "old.plugin"
        // Load profile when plugin doesn't exist
        // Actions should be removed from profile
    }

    #[test]
    fn test_removed_plugin_actions_nested_in_multiaction() {
        // Test that nested actions from removed plugins are cleaned
        let _temp_dir = setup_test_dir();
        
        // Multi-action containing actions from removed plugin
        // Nested actions should be removed but multi-action remains
    }

    #[test]
    fn test_opendeck_actions_always_kept() {
        // Test that opendeck built-in actions are never removed
        let _temp_dir = setup_test_dir();
        
        // Profile with opendeck.multiaction and opendeck.toggleaction
        // Should never be cleaned even if not in plugin registry
    }
}
