# OpenDeck Profile System Tests

This directory contains comprehensive tests for OpenDeck's profile system, covering creation, modification, actions, filesystem operations, and edge cases.

## Test Organization

The tests are organized into multiple files by functionality:

### 1. `profile_store_tests.rs`
Tests for core profile storage operations:
- **Store Basic Operations**: JSON serialization, file loading, recovery from temp/backup files
- **ProfileStores Operations**: Canonical IDs, profile creation, deletion, plugin filtering
- **Profile CRUD**: Creating, listing, and organizing profiles (including folder structures)
- **Device Config**: Selected profile persistence and fallback logic
- **Plugin Cleanup**: Handling removed plugins and missing actions

### 2. `action_instance_tests.rs`
Tests for action instance lifecycle:
- **Action Creation**: Creating simple actions, multi-actions, toggle-actions, nested actions
- **Action Removal**: Deleting actions, cleaning up images, handling nested removals
- **Action Movement**: Moving actions between slots, copying images, updating contexts
- **State Management**: Setting states, updating images for current profile
- **Multi-Action Behavior**: Child ordering, indices, nesting restrictions
- **Toggle-Action Behavior**: State-to-children synchronization, cycling
- **Profile Switching**: will_appear/will_disappear events, screen clearing

### 3. `filesystem_tests.rs`
Tests for filesystem state verification:
- **Filesystem Structure**: JSON file locations, directory organization, images per instance
- **Filesystem Cleanup**: Profile deletion, image cleanup, parent directory removal
- **Backup and Temp Files**: Save process, backup creation, recovery mechanisms
- **Disk Profile Format**: Relative paths, path normalization, data URL embedding
- **Concurrent Access**: File locking, multi-device scenarios

### 4. `edge_cases_tests.rs`
Tests for edge cases and stress scenarios:
- **Plugin Removal**: Handling removed plugins in various nesting scenarios
- **Missing/Corrupt Actions**: Invalid states, missing fields, array size mismatches
- **Path Separators**: Cross-platform path handling (Windows/Unix)
- **Boundary Conditions**: Zero keys/encoders, maximum positions, empty IDs
- **Stress Tests**: Large profiles, many nested actions, deep folders, rapid operations
- **Data Migration**: Old format profiles, different device sizes, export/import
- **Error Recovery**: Filesystem full, permission denied, interrupted saves
- **Validation**: JSON schema compliance, required fields

## Running Tests

### Run all tests:
```bash
cd src-tauri
cargo test
```

### Run specific test file:
```bash
cargo test --test profile_store_tests
cargo test --test action_instance_tests
cargo test --test filesystem_tests
cargo test --test edge_cases_tests
```

### Run specific test module:
```bash
cargo test store_basic_operations
cargo test plugin_removal_edge_cases
```

### Run specific test:
```bash
cargo test test_store_recovery_from_temp_file
```

### Run with output:
```bash
cargo test -- --nocapture
```

### Run with specific number of threads:
```bash
cargo test -- --test-threads=1
```

## Test Implementation Status

**Current Status**: All test files contain comprehensive test skeletons with clear documentation.

**Note**: Most tests are currently placeholders showing the structure and requirements. They need to be implemented with actual test logic that:
1. Accesses the internal Store and Profile types (may require making them public or using integration test patterns)
2. Initializes test environments with proper config directories
3. Verifies expected behavior and assertions
4. Cleans up test data after execution

## Implementation Roadmap

### Phase 1: Core Store Tests (Priority: HIGH)
- Implement `store_basic_operations` module
- Focus on save/load cycle and corruption recovery
- Verify backup file handling

### Phase 2: Profile CRUD Tests (Priority: HIGH)
- Implement profile creation and deletion tests
- Test folder organization and profile listing
- Verify filesystem cleanup

### Phase 3: Action Lifecycle Tests (Priority: MEDIUM)
- Implement action creation/removal tests
- Test multi-action and toggle-action behavior
- Verify image directory management

### Phase 4: Integration Tests (Priority: MEDIUM)
- Test profile switching workflows
- Test action movement and copying
- Test complex nesting scenarios

### Phase 5: Edge Cases and Stress Tests (Priority: LOW)
- Implement plugin removal scenarios
- Add corruption and recovery tests
- Add stress tests for large profiles

## Testing Best Practices

1. **Isolation**: Each test uses a temporary directory via `TempDir` to avoid interference
2. **Cleanup**: Tests should clean up all created resources (temp directories auto-cleanup)
3. **Assertions**: Verify both in-memory state and on-disk state after operations
4. **Documentation**: Each test includes a comment explaining what it validates
5. **Independence**: Tests should not depend on execution order
6. **Real Scenarios**: Tests should mirror actual user workflows where possible

## Key Testing Areas

### Profile Storage
- ✓ Creating new profiles with default values
- ✓ Saving profiles to disk with proper JSON formatting
- ✓ Loading profiles from disk
- ✓ Recovery from corrupt files using temp/backup files
- ✓ File locking during concurrent access
- ✓ Directory creation for nested profiles

### Action Instances
- ✓ Creating actions in empty slots
- ✓ Creating nested actions in multi-actions
- ✓ Removing actions and cleaning up resources
- ✓ Moving actions between slots
- ✓ Copying actions (retain=true)
- ✓ Updating action states and images
- ✓ Multi-action child management
- ✓ Toggle-action state synchronization

### Filesystem Operations
- ✓ Profile JSON files at correct paths
- ✓ Images directory per action instance
- ✓ Cleanup after profile deletion
- ✓ Cleanup after action removal
- ✓ Backup file creation during save
- ✓ Temp file cleanup after successful save
- ✓ Path separator normalization (cross-platform)

### Edge Cases
- ✓ Removed plugins and missing actions
- ✓ Corrupt JSON files
- ✓ Missing required fields
- ✓ Array size mismatches
- ✓ Invalid contexts
- ✓ Special characters in IDs
- ✓ Boundary conditions (zero keys, max positions)
- ✓ Large profiles with many actions

### Stress Testing
- ✓ Profiles with 100+ actions
- ✓ Deep folder nesting (5+ levels)
- ✓ Multi-actions with 50+ children
- ✓ Rapid profile switching
- ✓ Many concurrent operations
- ✓ Large settings JSON objects

## Contributing

When adding new tests:
1. Choose the appropriate test file based on functionality
2. Add tests to existing modules or create new modules
3. Follow naming conventions: `test_<what>_<scenario>`
4. Add clear documentation comments
5. Use helper functions to reduce duplication
6. Ensure tests are isolated and don't affect each other

## Known Limitations

- Some internal types (Store, ProfileStores) may need to be made public for testing
- Async operations require proper Tauri runtime setup in tests
- Device registration (DEVICES) may need mocking in tests
- Plugin registry access needs test infrastructure

## Future Enhancements

- Add property-based testing with `proptest`
- Add benchmarks for performance-critical operations
- Add fuzzing tests for JSON parsing
- Add integration tests with actual Tauri app context
- Add UI tests for profile management interface
