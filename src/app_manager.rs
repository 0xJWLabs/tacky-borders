use crate::border_manager::Border;
use crate::config_watcher::ConfigWatcher;
use crate::error::LogIfErr;
use crate::user_config::UserConfig;
use crate::windows_api::WindowsApi;
use anyhow::Context;
#[cfg(feature = "fast-hash")]
use fx_hash::{FxHashMap as HashMap, FxHashMapExt};
#[cfg(not(feature = "fast-hash"))]
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;
use std::sync::RwLock;
use std::sync::RwLockReadGuard;
use std::time::Duration;
use windows::core::Interface;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct2D::D2D1CreateFactory;
use windows::Win32::Graphics::Direct2D::ID2D1Device7;
use windows::Win32::Graphics::Direct2D::ID2D1Factory8;
use windows::Win32::Graphics::Direct2D::D2D1_FACTORY_TYPE_MULTI_THREADED;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_10_0;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_10_1;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_1;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_9_1;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_9_2;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_9_3;
use windows::Win32::Graphics::Direct3D11::D3D11CreateDevice;
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
use windows::Win32::Graphics::Direct3D11::D3D11_CREATE_DEVICE_BGRA_SUPPORT;
use windows::Win32::Graphics::Direct3D11::D3D11_SDK_VERSION;
use windows::Win32::Graphics::Dxgi::IDXGIDevice;

/// A global instance of the AppManager initialized lazily.
static APP_MANAGER: OnceLock<AppManager> = OnceLock::new();

/// AppManager is responsible for managing the application state, configuration, and devices.
#[derive(Debug)]
pub struct AppManager {
    /// Stores active borders keyed by their handles
    borders: Mutex<HashMap<isize, Border>>,
    /// Holds the handle of the currently active window
    active_window: Mutex<isize>,
    /// User configuration stored in a read-write lock
    config: RwLock<UserConfig>,
    /// Watches configuration file for changes
    config_watcher: RwLock<ConfigWatcher>,
    /// Flag to indicate whether active window polling is enabled
    is_polling_active_window: AtomicBool,
    /// Direct3D 11 device used for rendering
    device: ID3D11Device,
    /// DirectX Graphics Infrastructure device
    dxgi_device: IDXGIDevice,
    /// Direct2D device used for drawing
    d2d_device: ID2D1Device7,
}

unsafe impl Send for AppManager {}
unsafe impl Sync for AppManager {}

impl AppManager {
    /// Gets the global singleton instance of `AppManager`.
    pub fn get() -> &'static Self {
        APP_MANAGER.get_or_init(AppManager::new)
    }

    /// Returns a mutable lock guard for the borders map.
    pub fn borders(&self) -> MutexGuard<HashMap<isize, Border>> {
        self.borders.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Returns a mutable lock guard for the active window handle.
    pub fn active_window(&self) -> MutexGuard<isize> {
        self.active_window.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Sets the handle of the currently active window.
    pub fn set_active_window(&self, handle: isize) {
        if let Ok(mut active) = self.active_window.lock() {
            *active = handle;
        }
    }

    /// Stops the configuration file watcher.
    pub fn stop_config_watcher(&self) {
        if let Ok(mut config_watcher) = self.config_watcher.write() {
            config_watcher.stop().log_if_err();
        }
    }

    /// Starts the configuration file watcher.
    pub fn start_config_watcher(&self) {
        if let Ok(mut config_watcher) = self.config_watcher.write() {
            config_watcher.start().log_if_err();
        }
    }

    /// Returns whether the configuration file watcher is currently running.
    pub fn config_watcher_is_running(&self) -> bool {
        self.config_watcher.read().is_ok_and(|w| w.is_running())
    }

    /// Returns a read-only lock for the user configuration.
    pub fn config(&self) -> RwLockReadGuard<UserConfig> {
        self.config.read().unwrap()
    }

    /// Sets a new user configuration.
    pub fn set_config(&self, config: UserConfig) {
        if let Ok(mut cfg) = self.config.write() {
            *cfg = config;
        }
    }

    /// Returns a reference to the Direct3D device.
    pub fn device(&self) -> &ID3D11Device {
        &self.device
    }

    /// Returns a reference to the Direct2D device.
    pub fn d2d_device(&self) -> &ID2D1Device7 {
        &self.d2d_device
    }

    /// Returns a reference to the DXGI device.
    pub fn dxgi_device(&self) -> &IDXGIDevice {
        &self.dxgi_device
    }

    /// Returns whether the polling of the active window is enabled.
    pub fn is_polling_active_window(&self) -> bool {
        self.is_polling_active_window.load(Ordering::SeqCst)
    }

    /// Sets whether polling of the active window should be enabled.
    pub fn set_polling_active_window(&self, val: bool) {
        self.is_polling_active_window.store(val, Ordering::SeqCst);
    }

    /// Initializes a new AppManager instance, setting up configuration and DirectX devices.
    fn new() -> Self {
        let active_window = WindowsApi::get_foreground_window();

        let config_dir = UserConfig::get_config_dir().unwrap_or_default();
        let config_file = match UserConfig::detect_config_file(&config_dir) {
            Ok(file) => file,
            Err(_) => {
                println!("Creating default config file (AppManager)");
                UserConfig::create_default_config(&config_dir).unwrap_or_default()
            }
        };
        let mut config_watcher = ConfigWatcher::new(config_file, Duration::from_millis(200));

        let config = UserConfig::create().unwrap_or_else(|err| {
            error!("could not read config: {err:#}");
            UserConfig::default()
        });

        if config.monitor_config_changes {
            config_watcher.start().log_if_err();
        }

        let factory = unsafe {
            D2D1CreateFactory::<ID2D1Factory8>(D2D1_FACTORY_TYPE_MULTI_THREADED, None)
                .unwrap_or_else(|err| {
                    error!("could not create ID2D1Factory: {err}");
                    panic!()
                })
        };

        let (device, dxgi_device, d2d_device) =
            create_directx_devices(&factory).unwrap_or_else(|err| {
                error!("could not create directx devices: {err}");
                println!("could not create directx devices: {err}");
                panic!("could not create directx devices: {err}");
            });

        Self {
            borders: Mutex::new(HashMap::new()),
            active_window: Mutex::new(active_window),
            is_polling_active_window: AtomicBool::new(false),
            config: RwLock::new(config),
            config_watcher: RwLock::new(config_watcher),
            device,
            dxgi_device,
            d2d_device,
        }
    }
}

/// Helper function to create Direct3D and Direct2D devices.
fn create_directx_devices(
    factory: &ID2D1Factory8,
) -> anyhow::Result<(ID3D11Device, IDXGIDevice, ID2D1Device7)> {
    let creation_flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

    let feature_levels = [
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0,
        D3D_FEATURE_LEVEL_9_3,
        D3D_FEATURE_LEVEL_9_2,
        D3D_FEATURE_LEVEL_9_1,
    ];

    let mut device_opt: Option<ID3D11Device> = None;
    let mut feature_level: D3D_FEATURE_LEVEL = D3D_FEATURE_LEVEL::default();

    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            creation_flags,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device_opt),
            Some(&mut feature_level),
            None,
        )
    }?;

    debug!(
        "[create_directx_devices] DirectX device created successfully (feature level: {feature_level:X?})"
    );

    let device = device_opt.context("Could not get D3D11 device")?;
    let dxgi_device: IDXGIDevice = device.cast().context("ID3D11Device cast")?;
    let d2d_device =
        unsafe { factory.CreateDevice(&dxgi_device) }.context("Failed to create D2D device")?;

    Ok((device, dxgi_device, d2d_device))
}
