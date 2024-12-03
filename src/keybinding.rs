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

impl TryInto<HotkeyBinding> for &str {
    type Error = HotKeyParseError;

    fn try_into(self) -> Result<HotkeyBinding, Self::Error> {
        let tokens = self.split('+').collect::<Vec<&str>>();
        let mut modifiers = Vec::new();
        let mut key = None;

        match tokens.len() {
            1 => {
                key = Some(
                    VirtualKey::try_from(tokens[0].trim())
                        .map_err(|e| HotKeyParseError::UnsupportedKey(e.to_string()))?,
                );
            }
            _ => {
                for raw in tokens {
                    let token = raw.trim();

                    if token.is_empty() {
                        return Err(HotKeyParseError::EmptyToken(self.to_string()));
                    }

                    if key.is_some() {
                        return Err(HotKeyParseError::InvalidFormat(self.to_string()));
                    }

                    let temp_key = VirtualKey::try_from(token)
                        .map_err(|e| HotKeyParseError::UnsupportedKey(e.to_string()))?;

                    if let Ok(modifier) = temp_key.try_into() {
                        modifiers.push(modifier);
                    } else {
                        key = Some(temp_key);
                    }
                }
            }
        }

        Ok(HotkeyBinding::new(
            key.ok_or_else(|| HotKeyParseError::InvalidFormat(self.to_string()))?,
            Some(modifiers),
            None,
        ))
    }
}
