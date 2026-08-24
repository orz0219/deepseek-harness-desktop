fn main() {
    // Declare the application's custom commands so tauri-build autogenerates
    // `allow-<command>` / `deny-<command>` ACL permissions. This is what lets a
    // capability grant the injected dsh page (a *remote* origin) permission to
    // call `restore_archived_sessions` / `delete_archived_sessions` — without a
    // manifest, remote content can never reach custom commands (tauri IPC guard:
    // `plugin_command || has_app_acl_manifest || !is_local`).
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_status",
            "select_dsh",
            "restart_dsh",
            "restore_archived_sessions",
            "delete_archived_sessions",
            "get_file_tree",
            "get_git_diff",
            "read_file_content",
            "read_file_base64",
            "copy_to_clipboard",
            "reveal_in_finder",
        ]),
    ))
    .expect("failed to run tauri-build");
}
