use crate::border_manager::Border;
use crate::error::LogIfErr;
use crate::user_config::UserConfig;
use crate::user_config::UserConfigWatcher;
use crate::windows_api::WindowsApi;
use rustc_hash::FxHashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;
use windows::Win32::Graphics::Direct2D::D2D1CreateFactory;
use windows::Win32::Graphics::Direct2D::ID2D1Factory8;
use windows::Win32::Graphics::Direct2D::D2D1_FACTORY_TYPE_MULTI_THREADED;

pub static APP_STATE: LazyLock<AppState> = LazyLock::new(AppState::new);

pub struct AppState {
    pub borders: Mutex<FxHashMap<isize, Border>>,
    pub active_window: Mutex<isize>,
    pub config: RwLock<UserConfig>,
    pub config_watcher: RwLock<UserConfigWatcher>,
    pub render_factory: ID2D1Factory8,
    pub is_polling_active_window: AtomicBool,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    pub fn new() -> Self {
        let active_window = WindowsApi::get_foreground_window();

        let config_dir = UserConfig::get_config_dir().unwrap_or_default();
        let config_file = UserConfig::detect_config_file(&config_dir).unwrap_or_default();
        let mut config_watcher = UserConfigWatcher::new(config_file, Duration::from_millis(200));

        let config = match UserConfig::create() {
            Ok(config) => {
                if config.monitor_config_changes {
                    config_watcher.start().log_if_err();
                }
                config
            }
            Err(err) => {
                error!("could not read config: {err:#}");
                UserConfig::default() // Assuming `config_format` can have a default value
            }
        };

        let render_factory = unsafe {
            D2D1CreateFactory::<ID2D1Factory8>(D2D1_FACTORY_TYPE_MULTI_THREADED, None)
                .unwrap_or_else(|err| {
                    error!("could not create ID2D1Factory: {err}");
                    panic!()
                })
        };

        Self {
            borders: Mutex::new(FxHashMap::default()),
            active_window: Mutex::new(active_window),
            is_polling_active_window: AtomicBool::new(false),
            config: RwLock::new(config),
            config_watcher: RwLock::new(config_watcher),
            render_factory,
        }
    }

    pub fn is_polling_active_window(&self) -> bool {
        self.is_polling_active_window.load(Ordering::SeqCst)
    }

    pub fn set_polling_active_window(&self, val: bool) {
        self.is_polling_active_window.store(val, Ordering::SeqCst);
    }
}
