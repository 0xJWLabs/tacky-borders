#[allow(dead_code)]
use rustc_hash::FxHashSet;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{mpsc::Sender, Arc, Mutex};
use std::thread::spawn;
use win_binder::{listen, Event, EventType, Key};

type KeyBindingCallback = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct KeyBinding {
    pub name: String,
    pub virtual_key: Key,
    pub modifiers: Option<FxHashSet<Key>>,
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
        virtual_key: Key,
        modifiers: Option<Vec<Key>>,
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
    sender: Sender<KeyBinding>,
    receiver: Arc<Mutex<Receiver<KeyBinding>>>,
}

impl KeyBindingHook {
    pub fn new(keybinds: Option<Vec<KeyBinding>>) -> Self {
        let (sender, receiver) = channel();
        Self {
            bindings: Arc::new(Mutex::new(keybinds.unwrap_or_default())),
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn _add_binding(&self, binding: KeyBinding) {
        let mut bindings = self.bindings.lock().unwrap();
        bindings.push(binding);
    }

    pub fn listen(&self) {
        let active_modifiers = Arc::new(Mutex::new(FxHashSet::default()));

        let bindings = Arc::clone(&self.bindings);
        let sender = self.sender.clone();
        let receiver = self.receiver.clone();

        spawn(move || {
            if let Err(error) = listen(move |event: Event| match event.event_type {
                EventType::KeyPress(pressed_key) => {
                    let mut modifiers = active_modifiers.lock().unwrap();
                    update_active_modifiers(pressed_key, true, &mut modifiers);

                    let bindings = bindings.lock().unwrap();
                    for binding in &*bindings {
                        if binding.virtual_key == pressed_key {
                            if let Some(required_modifiers) = &binding.modifiers {
                                if required_modifiers.is_subset(&modifiers) {
                                    sender.send(binding.clone()).unwrap();
                                    modifiers.clear();
                                    break;
                                }
                            } else {
                                sender.send(binding.clone()).unwrap();
                                modifiers.clear();
                                break;
                            }
                        }
                    }
                }
                EventType::KeyRelease(released_key) => {
                    let mut modifiers = active_modifiers.lock().unwrap();
                    update_active_modifiers(released_key, false, &mut modifiers);
                }
                _ => {}
            }) {
                eprintln!("Error listening to global hotkeys: {:?}", error);
            }
        });

        spawn(move || {
            for binding in receiver.lock().unwrap().iter() {
                (binding.callback)();
            }
        });
    }
}

fn update_active_modifiers(key: Key, is_pressed: bool, modifiers: &mut FxHashSet<Key>) {
    if is_pressed {
        modifiers.insert(key);
    } else {
        modifiers.remove(&key);
    }
}
