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
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::RwLockReadGuard;
use std::time::Duration;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct2D::D2D1_FACTORY_TYPE_MULTI_THREADED;
use windows::Win32::Graphics::Direct2D::D2D1CreateFactory;
use windows::Win32::Graphics::Direct2D::ID2D1Device7;
use windows::Win32::Graphics::Direct2D::ID2D1Factory8;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_9_1;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_9_2;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_9_3;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_10_0;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_10_1;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_1;
use windows::Win32::Graphics::Direct3D11::D3D11_CREATE_DEVICE_BGRA_SUPPORT;
use windows::Win32::Graphics::Direct3D11::D3D11_SDK_VERSION;
use windows::Win32::Graphics::Direct3D11::D3D11CreateDevice;
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::core::Interface;

pub static APP: LazyLock<AppManager> = LazyLock::new(AppManager::new);

#[derive(Debug)]
pub struct AppManager {
    borders: Mutex<HashMap<isize, Border>>,
    active_window: Mutex<isize>,
    config: RwLock<UserConfig>,
    config_watcher: RwLock<ConfigWatcher>,
    device: ID3D11Device,
    dxgi_device: IDXGIDevice,
    d2d_device: ID2D1Device7,
    is_polling_active_window: AtomicBool,
}

unsafe impl Send for AppManager {}
unsafe impl Sync for AppManager {}

impl AppManager {
    pub fn device(&self) -> ID3D11Device {
        self.device.clone()
    }

    pub fn dxgi_device(&self) -> IDXGIDevice {
        self.dxgi_device.clone()
    }

    pub fn d2d_device(&self) -> ID2D1Device7 {
        self.d2d_device.clone()
    }

    pub fn borders(&self) -> MutexGuard<HashMap<isize, Border>> {
        self.borders.lock().unwrap()
    }

    pub fn active_window(&'static self) -> MutexGuard<'static, isize> {
        self.active_window.lock().unwrap()
    }
    
    pub fn set_active_window(&self, handle: isize) {
        *self.active_window.lock().unwrap() = handle;
    }

    pub fn stop_config_watcher(&self) {
        (self.config_watcher.write().unwrap())
        .stop()
        .log_if_err();
    }

    pub fn start_config_watcher(&self) {
        (self.config_watcher.write().unwrap())
        .start()
        .log_if_err();
    }

    pub fn config_watcher_is_running(&self) -> bool {
        (self.config_watcher.read().unwrap()).is_running()
    }

    pub fn config(&self) -> RwLockReadGuard<UserConfig> {
        self.config.read().unwrap()
    }

    pub fn set_config(&self, config: UserConfig) {
        *self.config.write().unwrap() = config;
    }

    pub fn new() -> Self {
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

        let create_directx_devices =
            || -> anyhow::Result<(ID3D11Device, IDXGIDevice, ID2D1Device7)> {
                let render_factory: ID2D1Factory8 = unsafe {
                    D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None).unwrap_or_else(
                        |err| {
                            error!("could not create ID2D1Factory: {err}");
                            panic!()
                        },
                    )
                };

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

                debug!("directx feature_level: {feature_level:X?}");

                let device = device_opt.context("could not get d3d11 device")?;
                let dxgi_device: IDXGIDevice = device.cast().context("id3d11device cast")?;
                let d2d_device =
                    unsafe { render_factory.CreateDevice(&dxgi_device) }.context("d2d_device")?;

                Ok((device, dxgi_device, d2d_device))
            };

        let (device, dxgi_device, d2d_device) = create_directx_devices().unwrap_or_else(|err| {
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

    pub fn is_polling_active_window(&self) -> bool {
        self.is_polling_active_window.load(Ordering::SeqCst)
    }

    pub fn set_polling_active_window(&self, val: bool) {
        self.is_polling_active_window.store(val, Ordering::SeqCst);
    }
}