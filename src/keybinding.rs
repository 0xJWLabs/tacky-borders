use rustc_hash::FxHashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use win_hotkey::keys::ModifiersKey;
use win_hotkey::keys::VirtualKey;
use win_hotkey::{HotkeyManager, HotkeyManagerImpl};

type HotkeyAction = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct HotkeyBinding {
    pub virtual_key: VirtualKey,
    pub modifiers: Option<Vec<ModifiersKey>>,
    pub action: Option<HotkeyAction>,
}

impl std::fmt::Debug for HotkeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotkeyBinding")
            .field("virtual_key", &self.virtual_key)
            .field("modifiers", &self.modifiers)
            .finish()
    }
}

impl HotkeyBinding {
    pub fn new(
        virtual_key: VirtualKey,
        modifiers: Option<Vec<ModifiersKey>>,
        action: Option<Box<dyn Fn() + Send + Sync + 'static>>, // Use Box to handle closures with dynamic dispatch
    ) -> Self {
        Self {
            virtual_key,
            modifiers: modifiers.map(|mods| mods.into_iter().collect()),
            action: action.map(|a| Arc::new(a) as HotkeyAction), // Wrap action in Option<Arc<dyn Fn() + Send + Sync>>
        }
    }

    #[allow(dead_code)]
    pub fn set_action(&mut self, action: Box<dyn Fn() + Send + Sync + 'static>) {
        self.action = Some(Arc::new(action));
    }
}

pub struct HotkeyHook {
    bindings: Arc<Mutex<FxHashMap<String, HotkeyBinding>>>,
    hotkey_manager: Arc<Mutex<HotkeyManager<()>>>,
    listening: Arc<AtomicBool>,
}

impl HotkeyHook {
    pub fn new(keybinds: Option<FxHashMap<String, HotkeyBinding>>) -> Self {
        let mut hotkey_manager = HotkeyManager::new();
        hotkey_manager.set_no_repeat(false);
        Self {
            bindings: Arc::new(Mutex::new(keybinds.unwrap_or_default())),
            hotkey_manager: Arc::new(Mutex::new(hotkey_manager)),
            listening: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn add_binding(&self, key: String, keybind: HotkeyBinding) {
        let mut bindings = self.bindings.lock().unwrap();
        bindings.insert(key, keybind);
    }

    pub fn start(&self) {
        if self.listening.load(Ordering::SeqCst) {
            eprintln!("Already listening for hotkeys.");
            return;
        }
        let bindings = self.bindings.clone();
        let hotkey_manager = self.hotkey_manager.clone();
        let listening = self.listening.clone();

        listening.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            // Lock bindings to access keybindings
            let mut hotkey_manager = hotkey_manager.lock().unwrap();
            let bindings = bindings.lock().unwrap();

            for binding in bindings.values() {
                // Check if the action exists before registering the hotkey
                if let Some(action) = &binding.action {
                    let action = action.clone();
                    // Lock hkm to mutate it
                    if let Err(e) = hotkey_manager.register(
                        binding.virtual_key,
                        binding.modifiers.as_deref(),
                        move || action(),
                    ) {
                        eprintln!(
                            "Failed to register keybinding {:?}: {:?}",
                            binding.virtual_key, e
                        );
                    }
                } else {
                    eprintln!(
                        "Hotkey {:?} does not have an associated action.",
                        binding.virtual_key
                    );
                }
            }

            // Event loop, will run until listening is set to false
            while listening.load(Ordering::SeqCst) {
                hotkey_manager.event_loop();
            }
        });
    }

    pub fn stop(&self) {
        if !self.listening.load(Ordering::SeqCst) {
            eprintln!("Not currently listening for hotkeys.");
            return;
        }

        // Set listening flag to false to stop the loop
        self.listening.store(false, Ordering::SeqCst);
        eprintln!("Stopped listening for hotkeys.");
    }
}

#[allow(non_snake_case)]
pub fn CreateHotkeyHook(keybinds: Option<FxHashMap<String, HotkeyBinding>>) -> HotkeyHook {
    HotkeyHook::new(keybinds)
}

#[allow(non_snake_case)]
pub fn RegisterHotkeyHook(
    hotkey_hook: &mut HotkeyHook,
    bindings: Option<FxHashMap<String, HotkeyBinding>>,
) {
    let bindings = bindings.unwrap_or_default();

    // Add all the bindings (if any)
    for (key, binding) in bindings {
        hotkey_hook.add_binding(key, binding);
    }

    hotkey_hook.start();
}

#[allow(non_snake_case)]
pub fn UnbindHotkeyHook(hotkey_hook: &mut HotkeyHook) {
    hotkey_hook.stop();
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum HotKeyParseError {
    UnsupportedKey(String),
    EmptyToken(String),
    InvalidFormat(String),
}

pub fn parse_hotkey(hotkey: &str) -> Result<HotkeyBinding, HotKeyParseError> {
    let tokens = hotkey.split('+').collect::<Vec<&str>>();
    let mut modifiers = Vec::new();
    let mut key = None;

    match tokens.len() {
        1 => {
            key = Some(parse_key(tokens[0])?);
        }
        _ => {
            for raw in tokens {
                let token = raw.trim();

                if token.is_empty() {
                    return Err(HotKeyParseError::EmptyToken(hotkey.to_string()));
                }

                if key.is_some() {
                    return Err(HotKeyParseError::InvalidFormat(hotkey.to_string()));
                }

                match token.to_uppercase().as_str() {
                    "OPTION" | "ALT" => modifiers.push(ModifiersKey::Alt),
                    "CONTROL" | "CTRL" => modifiers.push(ModifiersKey::Ctrl),
                    "SHIFT" => modifiers.push(ModifiersKey::Shift),
                    _ => {
                        key = Some(parse_key(token)?);
                    }
                }
            }
        }
    }

    Ok(HotkeyBinding::new(
        key.ok_or_else(|| HotKeyParseError::InvalidFormat(hotkey.to_string()))?,
        Some(modifiers),
        None,
    ))
}

fn parse_key(key: &str) -> Result<VirtualKey, HotKeyParseError> {
    use VirtualKey::*;

    match key.to_uppercase().as_str() {
        "BACK" => Ok(Back),
        "TAB" => Ok(Tab),
        "CLEAR" => Ok(Clear),
        "RETURN" => Ok(Return),
        "SHIFT" => Ok(Shift),
        "CONTROL" | "CTRL" => Ok(Control),
        "MENU" | "ALT" => Ok(Menu),
        "PAUSE" => Ok(Pause),
        "CAPITAL" => Ok(Capital),
        "ESC" | "ESCAPE" => Ok(Escape),
        "SPACE" => Ok(Space),
        "PRIOR" => Ok(Prior),
        "NEXT" => Ok(Next),
        "END" => Ok(End),
        "HOME" => Ok(Home),
        "LEFT" => Ok(Left),
        "UP" => Ok(Up),
        "RIGHT" => Ok(Right),
        "DOWN" => Ok(Down),
        "SELECT" => Ok(Select),
        "PRINT" => Ok(Print),
        "EXECUTE" => Ok(Execute),
        "SNAPSHOT" => Ok(Snapshot),
        "INSERT" => Ok(Insert),
        "DELETE" => Ok(Delete),
        "HELP" => Ok(Help),

        "LWIN" => Ok(LWin),
        "RWIN" => Ok(RWin),
        "APPS" => Ok(Apps),
        "SLEEP" => Ok(Sleep),

        "NUMPAD0" => Ok(Numpad0),
        "NUMPAD1" => Ok(Numpad1),
        "NUMPAD2" => Ok(Numpad2),
        "NUMPAD3" => Ok(Numpad3),
        "NUMPAD4" => Ok(Numpad4),
        "NUMPAD5" => Ok(Numpad5),
        "NUMPAD6" => Ok(Numpad6),
        "NUMPAD7" => Ok(Numpad7),
        "NUMPAD8" => Ok(Numpad8),
        "NUMPAD9" => Ok(Numpad9),

        "MULTIPLY" => Ok(Multiply),
        "ADD" => Ok(Add),
        "SEPARATOR" => Ok(Separator),
        "SUBTRACT" => Ok(Subtract),
        "DECIMAL" => Ok(Decimal),
        "DIVIDE" => Ok(Divide),

        "F1" => Ok(F1),
        "F2" => Ok(F2),
        "F3" => Ok(F3),
        "F4" => Ok(F4),
        "F5" => Ok(F5),
        "F6" => Ok(F6),
        "F7" => Ok(F7),
        "F8" => Ok(F8),
        "F9" => Ok(F9),
        "F10" => Ok(F10),
        "F11" => Ok(F11),
        "F12" => Ok(F12),
        "F13" => Ok(F13),
        "F14" => Ok(F14),
        "F15" => Ok(F15),
        "F16" => Ok(F16),
        "F17" => Ok(F17),
        "F18" => Ok(F18),
        "F19" => Ok(F19),
        "F20" => Ok(F20),
        "F21" => Ok(F21),
        "F22" => Ok(F22),
        "F23" => Ok(F23),
        "F24" => Ok(F24),

        "NUMLOCK" => Ok(Numlock),
        "SCROLL" => Ok(Scroll),

        "LSHIFT" => Ok(LShift),
        "RSHIFT" => Ok(RShift),
        "LCONTROL" => Ok(LControl),
        "RCONTROL" => Ok(RControl),
        "LMENU" => Ok(LMenu),
        "RMENU" => Ok(RMenu),

        "BROWSERBACK" => Ok(BrowserBack),
        "BROWSERFORWARD" => Ok(BrowserForward),
        "BROWSERREFRESH" => Ok(BrowserRefresh),
        "BROWSERSTOP" => Ok(BrowserStop),
        "BROWSERSEARCH" => Ok(BrowserSearch),
        "BROWSERFAVORITES" => Ok(BrowserFavorites),
        "BROWSERHOME" => Ok(BrowserHome),

        "VOLUMEMUTE" => Ok(VolumeMute),
        "VOLUMEDOWN" => Ok(VolumeDown),
        "VOLUMEUP" => Ok(VolumeUp),
        "MEDIANEXTTRACK" => Ok(MediaNextTrack),
        "MEDIAPREVTRACK" => Ok(MediaPrevTrack),
        "MEDIASTOP" => Ok(MediaStop),
        "MEDIAPLAYPAUSE" => Ok(MediaPlayPause),
        "LAUNCHMAIL" => Ok(LaunchMail),
        "LAUNCHMEDIASELECT" => Ok(LaunchMediaSelect),
        "LAUNCHAPP1" => Ok(LaunchApp1),
        "LAUNCHAPP2" => Ok(LaunchApp2),

        "OEM1" => Ok(Oem1),
        "OEMPLUS" => Ok(OemPlus),
        "OEMCOMMA" => Ok(OemComma),
        "OEMMINUS" => Ok(OemMinus),
        "OEMPERIOD" => Ok(OemPeriod),
        "OEM2" => Ok(Oem2),
        "OEM3" => Ok(Oem3),
        "OEM4" => Ok(Oem4),
        "OEM5" => Ok(Oem5),
        "OEM6" => Ok(Oem6),
        "OEM7" => Ok(Oem7),
        "OEM8" => Ok(Oem8),
        "OEM102" => Ok(Oem102),

        "ATTN" => Ok(Attn),
        "CRSEL" => Ok(Crsel),
        "EXSEL" => Ok(Exsel),
        "PLAY" => Ok(Play),
        "ZOOM" => Ok(Zoom),
        "PA1" => Ok(Pa1),
        "OEMCLEAR" => Ok(OemClear),

        "VK0" => Ok(Vk0),
        "VK1" => Ok(Vk1),
        "VK2" => Ok(Vk2),
        "VK3" => Ok(Vk3),
        "VK4" => Ok(Vk4),
        "VK5" => Ok(Vk5),
        "VK6" => Ok(Vk6),
        "VK7" => Ok(Vk7),
        "VK8" => Ok(Vk8),
        "VK9" => Ok(Vk9),

        "A" => Ok(A),
        "B" => Ok(B),
        "C" => Ok(C),
        "D" => Ok(D),
        "E" => Ok(E),
        "F" => Ok(F),
        "G" => Ok(G),
        "H" => Ok(H),
        "I" => Ok(I),
        "J" => Ok(J),
        "K" => Ok(K),
        "L" => Ok(L),
        "M" => Ok(M),
        "N" => Ok(N),
        "O" => Ok(O),
        "P" => Ok(P),
        "Q" => Ok(Q),
        "R" => Ok(R),
        "S" => Ok(S),
        "T" => Ok(T),
        "U" => Ok(U),
        "V" => Ok(V),
        "W" => Ok(W),
        "X" => Ok(X),
        "Y" => Ok(Y),
        "Z" => Ok(Z),

        key => Err(HotKeyParseError::UnsupportedKey(key.to_string())),
    }
}
