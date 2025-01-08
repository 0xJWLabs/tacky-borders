use anyhow::Result as AnyResult;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::CallNextHookEx;
use windows::Win32::UI::WindowsAndMessaging::SetWindowsHookExW;
use windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx;
use windows::Win32::UI::WindowsAndMessaging::HHOOK;
use windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT;
use windows::Win32::UI::WindowsAndMessaging::WH_KEYBOARD_LL;
use windows::Win32::UI::WindowsAndMessaging::WM_KEYDOWN;
use windows::Win32::UI::WindowsAndMessaging::WM_SYSKEYDOWN;

#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap()
    }};
}

use crate::sys_tray::SystemTrayEvent;
use crate::windows_api::PointerConversion;

const MODIFIER_KEYS: [u16; 6] = [
    VK_LSHIFT.0,
    VK_RSHIFT.0,
    VK_LCONTROL.0,
    VK_RCONTROL.0,
    VK_LMENU.0,
    VK_RMENU.0,
];

pub static KEYBOARD_HOOK: OnceLock<Arc<KeyboardHook>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ActiveKeybinding {
    pub vk_codes: Vec<u16>,
    pub config: KeybindingConfig,
}

#[derive(Debug)]
pub struct KeyboardHook {
    hook: Arc<Mutex<isize>>,
    keybindings_by_trigger_key: Arc<Mutex<FxHashMap<u16, Vec<ActiveKeybinding>>>>,
}

#[derive(Clone)]
pub struct KeybindingConfig {
    pub name: String,
    pub keybind: String,
    pub event: Option<SystemTrayEvent>,
}

impl KeybindingConfig {
    pub fn new(name: &str, keybind: &str, event: Option<SystemTrayEvent>) -> Self {
        Self {
            name: name.to_string(),
            keybind: keybind.to_string(),
            event,
        }
    }
}

impl core::fmt::Debug for KeybindingConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Display the name and keybind
        f.debug_struct("KeybindingConfig")
            .field("name", &self.name)
            .field("keybind", &self.keybind)
            .field(
                "event_callback",
                &self
                    .event
                    .as_ref()
                    .map_or("None", |action| action.as_function_name()),
            )
            .finish()
    }
}

impl KeyboardHook {
    /// Creates an instance of `KeyboardHook`.
    pub fn new(keybindings: &Vec<KeybindingConfig>) -> AnyResult<Arc<Self>> {
        let keyboard_hook = Arc::new(Self {
            hook: Arc::new(Mutex::new(isize::default())),
            keybindings_by_trigger_key: Arc::new(Mutex::new(Self::keybindings_by_trigger_key(
                keybindings,
            ))),
        });

        KEYBOARD_HOOK
            .set(keyboard_hook.clone())
            .map_err(|_| anyhow::anyhow!("Keyboard hook already running."))?;

        Ok(keyboard_hook)
    }

    /// Starts a keyboard hook on the current thread.
    ///
    /// Assumes that a message loop is currently running.
    pub fn start(&self) -> AnyResult<()> {
        *self.hook.lock().unwrap() =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0) }?
                .0
                .as_int();

        Ok(())
    }

    pub fn update(&self, keybindings: &[KeybindingConfig]) {
        *self.keybindings_by_trigger_key.lock().unwrap() =
            Self::keybindings_by_trigger_key(&keybindings.to_owned());
    }

    /// Stops the low-level keyboard hook.
    pub fn stop(&self) -> AnyResult<()> {
        unsafe { UnhookWindowsHookEx(HHOOK(self.hook.lock().unwrap().as_ptr())) }?;
        Ok(())
    }

    fn keybindings_by_trigger_key(
        keybindings: &Vec<KeybindingConfig>,
    ) -> FxHashMap<u16, Vec<ActiveKeybinding>> {
        let mut keybinding_map = FxHashMap::default();

        for keybinding in keybindings {
            let binding = keybinding.keybind.clone();

            let vk_codes = Self::extract_vk_codes(&binding);

            // Safety: A split string always has at least one element.
            let trigger_key = *vk_codes.last().unwrap();

            keybinding_map
                .entry(trigger_key)
                .or_insert_with(Vec::new)
                .push(ActiveKeybinding {
                    vk_codes,
                    config: keybinding.clone(),
                });
        }

        keybinding_map
    }

    // Emits a keybinding callback if a keybinding should be triggered.
    ///
    /// Returns `true` if the callback should be blocked.
    fn handle_key_event(&self, vk_code: u16) -> bool {
        let keybindings = self
            .keybindings_by_trigger_key
            .lock()
            .unwrap()
            .get(&vk_code)
            .cloned();

        if let Some(keybindings) = keybindings {
            let mut cached_key_states = FxHashMap::default();
            if let Some(longest_keybinding) =
                Self::match_keybindings(vk_code, &keybindings, &mut cached_key_states)
            {
                // Get the modifier keys to reject based on the longest matching
                // keybinding.
                let mut modifier_keys_to_reject =
                    Self::get_modifier_keys_to_reject(&longest_keybinding);

                // Check if any modifier keys to reject are currently down.
                let has_modifier_keys_to_reject = Self::has_conflicting_modifiers(
                    &mut modifier_keys_to_reject,
                    &mut cached_key_states,
                );

                if has_modifier_keys_to_reject {
                    return false;
                }

                if let Some(event) = longest_keybinding.config.event {
                    event.execute();
                }

                return true;
            } else {
                return false;
            }
        }

        false
    }

    // Matches the longest keybinding for a given `vk_code` and `keybindings`.
    fn match_keybindings(
        vk_code: u16,
        keybindings: &[ActiveKeybinding],
        cached_key_states: &mut FxHashMap<u16, bool>,
    ) -> Option<ActiveKeybinding> {
        keybindings
            .iter()
            .filter(|keybinding| Self::is_keybinding_active(vk_code, keybinding, cached_key_states))
            .max_by_key(|keybinding| keybinding.vk_codes.len())
            .cloned()
    }

    /// Gets modifier keys that should be rejected based on the active keybinding.
    fn get_modifier_keys_to_reject(
        longest_keybinding: &ActiveKeybinding,
    ) -> impl Iterator<Item = u16> + '_ {
        MODIFIER_KEYS.iter().filter_map(move |&modifier_key| {
            if !longest_keybinding.vk_codes.contains(&modifier_key)
                && !longest_keybinding
                    .vk_codes
                    .contains(&Self::generic_key(modifier_key))
            {
                Some(modifier_key)
            } else {
                None
            }
        })
    }

    /// Checks if any modifier keys to reject are currently down.
    fn has_conflicting_modifiers(
        modifier_keys_to_reject: &mut impl Iterator<Item = u16>,
        cached_key_states: &mut FxHashMap<u16, bool>,
    ) -> bool {
        modifier_keys_to_reject.any(|modifier_key| {
            if let Some(&is_key_down) = cached_key_states.get(&modifier_key) {
                is_key_down
            } else {
                Self::is_key_down(modifier_key)
            }
        })
    }

    fn extract_vk_codes(binding: &str) -> Vec<u16> {
        binding
        .split('+')
        .filter_map(|key| {
            let vk_code = Self::key_to_vk_code(key);
            if vk_code.is_none() {
                warn!(
                    "Unrecognized key on current keyboard '{}'. Ensure that alt or shift isn't required for the key.",
                    key
                );
            }
            vk_code
        })
        .collect()
    }

    /// Gets the generic key code for a given key code.
    fn generic_key(key: u16) -> u16 {
        match VIRTUAL_KEY(key) {
            VK_LMENU | VK_RMENU => VK_MENU.0,
            VK_LSHIFT | VK_RSHIFT => VK_SHIFT.0,
            VK_LCONTROL | VK_RCONTROL => VK_CONTROL.0,
            _ => key,
        }
    }

    /// Gets whether the specified key is currently down.
    fn is_key_down(key: u16) -> bool {
        match VIRTUAL_KEY(key) {
            VK_MENU => Self::is_key_down_raw(VK_LMENU.0) || Self::is_key_down_raw(VK_RMENU.0),
            VK_SHIFT => Self::is_key_down_raw(VK_LSHIFT.0) || Self::is_key_down_raw(VK_RSHIFT.0),
            VK_CONTROL => {
                Self::is_key_down_raw(VK_LCONTROL.0) || Self::is_key_down_raw(VK_RCONTROL.0)
            }
            _ => Self::is_key_down_raw(key),
        }
    }

    /// Gets whether the specified key is currently down using the raw key
    /// code.
    fn is_key_down_raw(key: u16) -> bool {
        unsafe { (GetKeyState(key.into()) & 0x80) == 0x80 }
    }

    /// Checks if a keybinding is active based on the given `vk_code` and cached key states.
    ///
    /// Returns `true` if keybinding is active.
    fn is_keybinding_active(
        vk_code: u16,
        keybinding: &ActiveKeybinding,
        cached_key_states: &mut FxHashMap<u16, bool>,
    ) -> bool {
        keybinding.vk_codes.iter().all(|&key| {
            if key == vk_code {
                true
            } else {
                *cached_key_states
                    .entry(key)
                    .or_insert_with(|| Self::is_key_down(key))
            }
        })
    }

    fn key_to_vk_code(key: &str) -> Option<u16> {
        match key.to_lowercase().as_str() {
            "a" => Some(VK_A.0),
            "b" => Some(VK_B.0),
            "c" => Some(VK_C.0),
            "d" => Some(VK_D.0),
            "e" => Some(VK_E.0),
            "f" => Some(VK_F.0),
            "g" => Some(VK_G.0),
            "h" => Some(VK_H.0),
            "i" => Some(VK_I.0),
            "j" => Some(VK_J.0),
            "k" => Some(VK_K.0),
            "l" => Some(VK_L.0),
            "m" => Some(VK_M.0),
            "n" => Some(VK_N.0),
            "o" => Some(VK_O.0),
            "p" => Some(VK_P.0),
            "q" => Some(VK_Q.0),
            "r" => Some(VK_R.0),
            "s" => Some(VK_S.0),
            "t" => Some(VK_T.0),
            "u" => Some(VK_U.0),
            "v" => Some(VK_V.0),
            "w" => Some(VK_W.0),
            "x" => Some(VK_X.0),
            "y" => Some(VK_Y.0),
            "z" => Some(VK_Z.0),
            "0" | "d0" => Some(VK_0.0),
            "1" | "d1" => Some(VK_1.0),
            "2" | "d2" => Some(VK_2.0),
            "3" | "d3" => Some(VK_3.0),
            "4" | "d4" => Some(VK_4.0),
            "5" | "d5" => Some(VK_5.0),
            "6" | "d6" => Some(VK_6.0),
            "7" | "d7" => Some(VK_7.0),
            "8" | "d8" => Some(VK_8.0),
            "9" | "d9" => Some(VK_9.0),
            "numpad0" => Some(VK_NUMPAD0.0),
            "numpad1" => Some(VK_NUMPAD1.0),
            "numpad2" => Some(VK_NUMPAD2.0),
            "numpad3" => Some(VK_NUMPAD3.0),
            "numpad4" => Some(VK_NUMPAD4.0),
            "numpad5" => Some(VK_NUMPAD5.0),
            "numpad6" => Some(VK_NUMPAD6.0),
            "numpad7" => Some(VK_NUMPAD7.0),
            "numpad8" => Some(VK_NUMPAD8.0),
            "numpad9" => Some(VK_NUMPAD9.0),
            "f1" => Some(VK_F1.0),
            "f2" => Some(VK_F2.0),
            "f3" => Some(VK_F3.0),
            "f4" => Some(VK_F4.0),
            "f5" => Some(VK_F5.0),
            "f6" => Some(VK_F6.0),
            "f7" => Some(VK_F7.0),
            "f8" => Some(VK_F8.0),
            "f9" => Some(VK_F9.0),
            "f10" => Some(VK_F10.0),
            "f11" => Some(VK_F11.0),
            "f12" => Some(VK_F12.0),
            "f13" => Some(VK_F13.0),
            "f14" => Some(VK_F14.0),
            "f15" => Some(VK_F15.0),
            "f16" => Some(VK_F16.0),
            "f17" => Some(VK_F17.0),
            "f18" => Some(VK_F18.0),
            "f19" => Some(VK_F19.0),
            "f20" => Some(VK_F20.0),
            "f21" => Some(VK_F21.0),
            "f22" => Some(VK_F22.0),
            "f23" => Some(VK_F23.0),
            "f24" => Some(VK_F24.0),
            "shift" | "shiftkey" => Some(VK_SHIFT.0),
            "lshift" | "lshiftkey" => Some(VK_LSHIFT.0),
            "rshift" | "rshiftkey" => Some(VK_RSHIFT.0),
            "ctrl" | "controlkey" | "control" => Some(VK_CONTROL.0),
            "lctrl" | "lcontrolkey" => Some(VK_LCONTROL.0),
            "rctrl" | "rcontrolkey" => Some(VK_RCONTROL.0),
            "alt" | "menu" => Some(VK_MENU.0),
            "lalt" | "lmenu" => Some(VK_LMENU.0),
            "ralt" | "rmenu" => Some(VK_RMENU.0),
            "lwin" => Some(VK_LWIN.0),
            "rwin" => Some(VK_RWIN.0),
            "space" => Some(VK_SPACE.0),
            "escape" => Some(VK_ESCAPE.0),
            "back" => Some(VK_BACK.0),
            "tab" => Some(VK_TAB.0),
            "enter" | "return" => Some(VK_RETURN.0),
            "left" => Some(VK_LEFT.0),
            "right" => Some(VK_RIGHT.0),
            "up" => Some(VK_UP.0),
            "down" => Some(VK_DOWN.0),
            "num_lock" => Some(VK_NUMLOCK.0),
            "scroll_lock" => Some(VK_SCROLL.0),
            "caps_lock" => Some(VK_CAPITAL.0),
            "page_up" => Some(VK_PRIOR.0),
            "page_down" => Some(VK_NEXT.0),
            "insert" => Some(VK_INSERT.0),
            "delete" => Some(VK_DELETE.0),
            "end" => Some(VK_END.0),
            "home" => Some(VK_HOME.0),
            "print_screen" => Some(VK_SNAPSHOT.0),
            "multiply" => Some(VK_MULTIPLY.0),
            "add" => Some(VK_ADD.0),
            "subtract" => Some(VK_SUBTRACT.0),
            "decimal" => Some(VK_DECIMAL.0),
            "divide" => Some(VK_DIVIDE.0),
            "volume_up" => Some(VK_VOLUME_UP.0),
            "volume_down" => Some(VK_VOLUME_DOWN.0),
            "volume_mute" => Some(VK_VOLUME_MUTE.0),
            "media_next_track" => Some(VK_MEDIA_NEXT_TRACK.0),
            "media_prev_track" => Some(VK_MEDIA_PREV_TRACK.0),
            "media_stop" => Some(VK_MEDIA_STOP.0),
            "media_play_pause" => Some(VK_MEDIA_PLAY_PAUSE.0),
            "oem_semicolon" => Some(VK_OEM_1.0),
            "oem_question" => Some(VK_OEM_2.0),
            "oem_tilde" => Some(VK_OEM_3.0),
            "oem_open_brackets" => Some(VK_OEM_4.0),
            "oem_pipe" => Some(VK_OEM_5.0),
            "oem_close_brackets" => Some(VK_OEM_6.0),
            "oem_quotes" => Some(VK_OEM_7.0),
            "oem_plus" => Some(VK_OEM_PLUS.0),
            "oem_comma" => Some(VK_OEM_COMMA.0),
            "oem_minus" => Some(VK_OEM_MINUS.0),
            "oem_period" => Some(VK_OEM_PERIOD.0),
            _ => {
                // Check if the key exists on the current keyboard layout.
                let utf16_key = key.encode_utf16().next()?;
                let layout = unsafe { GetKeyboardLayout(0) };
                let vk_code = unsafe { VkKeyScanExW(utf16_key, layout) };

                if vk_code == -1 {
                    return None;
                }

                // The low-order byte contains the virtual-key code and the high-
                // order byte contains the shift state.
                let [high_order, low_order] = vk_code.to_be_bytes();

                // Key is valid if it doesn't require shift or alt to be pressed.
                match high_order {
                    0 => Some(low_order as u16),
                    _ => None,
                }
            }
        }
    }
}

extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let should_ignore =
        code != 0 || !(wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN);

    // If the code is less than zero, the hook procedure must pass the hook
    // notification directly to other applications. We also only care about
    // keydown events.
    if should_ignore {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    // Get struct with keyboard input event.
    let input = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };

    if let Some(hook) = KEYBOARD_HOOK.get() {
        let should_block = hook.handle_key_event(input.vkCode as u16);

        if should_block {
            return LRESULT(1);
        }
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
