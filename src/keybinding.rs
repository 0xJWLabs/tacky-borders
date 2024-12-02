#[allow(dead_code)]
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use win_hotkey::keys::ModifiersKey;
use win_hotkey::keys::VirtualKey;
use win_hotkey::{HotkeyManager, HotkeyManagerImpl};

type KeyBindingCallback = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct KeyBinding {
    pub name: String,
    pub virtual_key: VirtualKey,
    pub modifiers: Option<Vec<ModifiersKey>>,
    pub callback: KeyBindingCallback,
}

impl std::fmt::Debug for KeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyBinding")
            .field("name", &self.name)
            .field("virtual_key", &self.virtual_key)
            .field("modifiers", &self.modifiers)
            .finish()
    }
}

impl KeyBinding {
    pub fn new(
        name: String,
        virtual_key: VirtualKey,
        modifiers: Option<Vec<ModifiersKey>>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            name,
            virtual_key,
            modifiers: modifiers.map(|mods| mods.into_iter().collect()),
            callback: Arc::new(callback),
        }
    }
}

pub struct KeyBindingHook {
    bindings: Arc<Mutex<Vec<KeyBinding>>>,
    hkm: Arc<Mutex<HotkeyManager<()>>>,
}

impl KeyBindingHook {
    pub fn new(keybinds: Option<Vec<KeyBinding>>) -> Self {
        let mut hkm = HotkeyManager::new();
        hkm.set_no_repeat(false);
        Self {
            bindings: Arc::new(Mutex::new(keybinds.unwrap_or_default())),
            hkm: Arc::new(Mutex::new(hkm)),
        }
    }

    pub fn add_binding(&self, keybind: KeyBinding) {
        let mut bindings = self.bindings.lock().unwrap();
        bindings.push(keybind);
    }

    pub fn listen(&self) {
        let bindings = Arc::clone(&self.bindings);
        let hkm = Arc::clone(&self.hkm);

        thread::spawn(move || {
            // Lock bindings to access keybindings
            let mut hkm = hkm.lock().unwrap();
            let bindings = bindings.lock().unwrap();
            for binding in bindings.iter() {
                let callback = Arc::clone(&binding.callback);
                // Lock hkm to mutate it

                if let Err(e) = hkm.register(
                    binding.virtual_key,
                    binding.modifiers.as_deref(),
                    move || callback(),
                ) {
                    eprintln!("Failed to register keybinding {}: {:?}", binding.name, e);
                }
            }
            // Event loop will run after all bindings are registered
            hkm.event_loop();
        });
    }
}
