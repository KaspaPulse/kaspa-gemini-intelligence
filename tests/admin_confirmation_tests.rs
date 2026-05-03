use kaspa_pulse::presentation::telegram::handlers::admin_confirm::{
    action_from_admin_do_callback, sensitive_action_from_callback,
    sensitive_action_from_toggle_flag, SensitiveAdminAction,
};

#[test]
fn sensitive_admin_callbacks_are_detected() {
    assert_eq!(
        sensitive_action_from_callback("cmd_pause"),
        Some(SensitiveAdminAction::Pause)
    );
    assert_eq!(
        sensitive_action_from_callback("cmd_resume"),
        Some(SensitiveAdminAction::Resume)
    );
    assert_eq!(
        sensitive_action_from_callback("cmd_restart"),
        Some(SensitiveAdminAction::Restart)
    );
    assert_eq!(
        sensitive_action_from_callback("cmd_cleanup_events"),
        Some(SensitiveAdminAction::CleanupEvents)
    );
    assert_eq!(
        sensitive_action_from_callback("confirm_forget_all"),
        Some(SensitiveAdminAction::ForgetAll)
    );
}

#[test]
fn sensitive_toggle_flags_are_detected() {
    assert_eq!(
        sensitive_action_from_toggle_flag("MAINTENANCE"),
        Some(SensitiveAdminAction::ToggleMaintenance)
    );
    assert_eq!(
        sensitive_action_from_toggle_flag("SYNC"),
        Some(SensitiveAdminAction::ToggleLiveSync)
    );
    assert_eq!(
        sensitive_action_from_toggle_flag("MEMORY"),
        Some(SensitiveAdminAction::ToggleMemoryCleaner)
    );
}

#[test]
fn confirmed_actions_rewrite_to_original_callbacks() {
    assert_eq!(SensitiveAdminAction::Pause.execute_callback(), "cmd_pause");
    assert_eq!(
        SensitiveAdminAction::ToggleMaintenance.execute_callback(),
        "btn_toggle_MAINTENANCE_MODE"
    );
    assert_eq!(
        SensitiveAdminAction::ForgetAll.execute_callback(),
        "do_forget_all"
    );
}

#[test]
fn invalid_admin_do_callbacks_are_rejected() {
    assert!(action_from_admin_do_callback("admin_do:bad:token").is_err());
    assert!(action_from_admin_do_callback("bad").is_err());
}
