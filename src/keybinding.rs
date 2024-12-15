use rustc_hash::FxHashMap;
use win_hotkey::global::GlobalHotkey;
use win_hotkey::global::GlobalHotkeyManager;
use win_hotkey::global::GlobalHotkeyManagerImpl;

#[allow(non_snake_case)]
pub fn CreateHotkeyHook() -> GlobalHotkeyManager<()> {
    GlobalHotkeyManager::new()
}

#[allow(non_snake_case)]
pub fn RegisterHotkeyHook(
    hook: &GlobalHotkeyManager<()>,
    bindings: Option<FxHashMap<String, GlobalHotkey<()>>>,
) {
    let bindings = bindings.unwrap_or_default();

    // Add all the bindings (if any)
    for (key, binding) in bindings {
        hook.add_hotkey(key, binding);
    }

    hook.start();
}

#[allow(non_snake_case)]
pub fn UnbindHotkeyHook(hotkey_hook: &GlobalHotkeyManager<()>) {
    let stopped = hotkey_hook.stop();
    if stopped {
        debug!("unbind hotkey");
    }
}
