# Accessibility

Cross-platform accessibility middleware. Owns macOS AX / CGEvent / IOKit FFI, the unified Swift helper-process bridge, focused-text inspection, system-permission detection (Accessibility, Input Monitoring, Microphone), the Globe-key listener, "System Events" automation-denial tracking, terminal heuristics, and AX-string normalization. Centralises platform-specific code so that `voice` never touches FFI directly.

## Public surface

Re-exported from `mod.rs`:

- `pub fn clear_automation_denial` / `mark_system_events_denied` / `system_events_denied` — `automation_state.rs` — tracks whether macOS has denied AppleScript "System Events" automation, so callers can stop retrying until the user re-grants it.
- `pub fn focused_text_context` / `focused_text_context_verbose` / `validate_focused_target` — `focus.rs` — query the OS for the currently focused text field.
- `pub fn globe_listener_start` / `globe_listener_stop` / `globe_listener_poll` / `pub struct GlobeHotkeyPollResult` / `pub enum GlobeHotkeyStatus` — `globe.rs` — macOS Globe-key (Fn) hotkey monitor.
- `pub fn precompile_helper_background` — split across `helper_part_01.rs` and `helper_part_02.rs` (re-exported via `helper.rs`) — warm the Swift helper process at startup.
- Permission detection: `detect_permissions`, `detect_microphone_permission`, `microphone_denied_message`, `permission_to_str`, `request_microphone_access` (cross-platform); macOS-only `detect_accessibility_permission`, `detect_input_monitoring_permission`, `open_macos_privacy_pane`, `request_accessibility_access` — `permissions.rs`.
- `pub fn extract_terminal_input_context` / `is_terminal_app` / `is_text_role` / `looks_like_terminal_buffer` — `terminal.rs` — terminal-window heuristics.
- `pub fn normalize_ax_value` / `parse_ax_number` / `truncate_tail` — `text_util.rs` — AX value normalization.
- `pub struct ElementBounds` / `FocusedTextContext` / `PermissionKind` / `PermissionState` / `PermissionStatus` — `types.rs`.

## Calls into

- macOS frameworks (`ApplicationServices`, `CoreGraphics`, `IOKit`, `AVFoundation`) via FFI.
- Bundled Swift helper process for AX queries that require a separate process.
- `crates/openhuman-core/src/config/` — overlay sizing and helper paths (light dependency).

## Called by

- `crates/openhuman-core/src/voice/` (`server_part_01.rs`, `always_on_part_02.rs`, `audio_capture.rs`, `text_input.rs`) — microphone permission, focused-text, and terminal-context helpers.
- `crates/openhuman-core/src/core/all_tests.rs` — registry-level test coverage.

## Tests

- Permission and focus coverage runs through `permissions_tests.rs`, inline module tests, and retained consumers.
- AX FFI surface is best validated end-to-end on a real macOS host — most CI runs are Linux and skip platform-gated paths.
