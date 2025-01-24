use crate::animation::manager::AnimationManager;
use crate::animation::wrapper::AnimationEngineVec;
use crate::app_manager::AppManager;
use crate::colors::Color;
use crate::colors::ColorImpl;
use crate::core::animation::AnimationKind;
use crate::core::rect::Rect;
use crate::effect::manager::EffectManager;
use crate::effect::wrapper::EffectEngineVec;
use crate::error::LogIfErr;
use crate::parsed_config::ParsedConfig;
use crate::parsed_config::WindowRule;
use crate::render_resources::RenderResources;
use crate::windows_api::HWNDConversion;
use crate::windows_api::PointerConversion;
use crate::windows_api::ToWideString;
use crate::windows_api::WM_APP_FOREGROUND;
use crate::windows_api::WM_APP_HIDECLOAKED;
use crate::windows_api::WM_APP_LOCATIONCHANGE;
use crate::windows_api::WM_APP_MINIMIZEEND;
use crate::windows_api::WM_APP_MINIMIZESTART;
use crate::windows_api::WM_APP_REORDER;
use crate::windows_api::WM_APP_SHOWUNCLOAKED;
use crate::windows_api::WM_APP_TIMER;
use crate::windows_api::WindowsApi;
use anyhow::Context;
use anyhow::anyhow;
use std::thread;
use std::time;
use std::time::Instant;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::D2DERR_RECREATE_TARGET;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::RECT;
use windows::Win32::Foundation::S_OK;
use windows::Win32::Foundation::TRUE;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COMPOSITE_MODE_SOURCE_OVER;
use windows::Win32::Graphics::Direct2D::D2D1_BRUSH_PROPERTIES;
use windows::Win32::Graphics::Direct2D::D2D1_INTERPOLATION_MODE_LINEAR;
use windows::Win32::Graphics::Direct2D::D2D1_ROUNDED_RECT;
use windows::Win32::Graphics::Direct2D::ID2D1Brush;
use windows::Win32::Graphics::Direct2D::ID2D1CommandList;
use windows::Win32::Graphics::Direct2D::ID2D1DeviceContext7;
use windows::Win32::Graphics::Dwm::DWM_BB_BLURREGION;
use windows::Win32::Graphics::Dwm::DWM_BB_ENABLE;
use windows::Win32::Graphics::Dwm::DWM_BLURBEHIND;
use windows::Win32::Graphics::Dwm::DwmEnableBlurBehindWindow;
use windows::Win32::Graphics::Dxgi::DXGI_PRESENT;
use windows::Win32::Graphics::Gdi::CreateRectRgn;
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
use windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;
use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
use windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA;
use windows::Win32::UI::WindowsAndMessaging::LWA_COLORKEY;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS;
use windows::Win32::UI::WindowsAndMessaging::SM_CXVIRTUALSCREEN;
use windows::Win32::UI::WindowsAndMessaging::SWP_HIDEWINDOW;
use windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WM_CREATE;
use windows::Win32::UI::WindowsAndMessaging::WM_NCDESTROY;
use windows::Win32::UI::WindowsAndMessaging::WM_PAINT;
use windows::Win32::UI::WindowsAndMessaging::WM_QUIT;
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGED;
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGING;
use windows::core::CloneType;
use windows::core::TypeKind;

use super::get_active_window;
use super::window_border;
use super::window_borders;

impl TypeKind for Border {
    type TypeKind = CloneType;
}

impl Eq for Border {}

impl PartialEq for Border {
    fn eq(&self, other: &Self) -> bool {
        self.tracking_window as usize == other.tracking_window as usize
    }
}

#[derive(Debug, Default, Clone)]
pub struct Border {
    pub border_window: isize,
    pub tracking_window: isize,
    pub is_window_active: bool,
    pub window_rect: Rect,
    pub window_padding: i32,
    pub render_rect: D2D1_ROUNDED_RECT,
    pub width: i32,
    pub offset: i32,
    pub radius: f32,
    pub current_monitor: HMONITOR,
    pub current_dpi: f32,
    pub render_resources: RenderResources,
    pub active_color: Color,
    pub inactive_color: Color,
    pub animation_manager: AnimationManager,
    pub effect_manager: EffectManager,
    pub last_render_time: Option<Instant>,
    pub initialize_delay: u32,
    pub unminimize_delay: u32,
    pub pause: bool,
    pub process_name: String,
}

impl Border {
    pub fn border_window(&self) -> HWND {
        self.border_window.as_hwnd()
    }

    pub fn tracking_window(&self) -> HWND {
        self.tracking_window.as_hwnd()
    }

    pub fn from_optional(handle: isize) -> Option<Border> {
        // Check if the border already exists.
        if let Some(existing_border) = window_border(handle) {
            return Some(existing_border);
        }

        // Ensure the window is visible on screen and is a top-level window.
        if !WindowsApi::is_window_visible_on_screen(handle)
            || !WindowsApi::is_window_top_level(handle)
        {
            return None;
        }

        // Retrieve window-specific rules.
        let window_rule = WindowsApi::get_window_rule(handle);

        // Handle border creation based on the rule's enabled status.
        match window_rule.match_window.enabled {
            Some(false) => {
                info!(
                    "Border creation is disabled for window: {:?}",
                    handle.as_hwnd()
                );
                None
            }
            Some(true) | None if !WindowsApi::has_filtered_style(handle) => {
                Border::create(handle, window_rule);
                None
            }
            _ => None,
        }
    }

    pub fn show(handle: isize) {
        // Check if the border already exists for the given window.
        if let Some(existing_border) = window_border(handle) {
            // Post a 'SHOW' message to make the existing border visible.
            if let Err(e) = WindowsApi::post_message_w(
                Some(existing_border.border_window.as_hwnd()),
                WM_APP_SHOWUNCLOAKED,
                WPARAM(0),
                LPARAM(0),
            ) {
                error!("failed to post WM_APP_SHOW_UNCLOAKED message: {:?}", e);
            }
            return;
        }

        // Ensure the window is visible on screen and is a top-level window.
        if !WindowsApi::is_window_visible_on_screen(handle)
            || !WindowsApi::is_window_top_level(handle)
        {
            return;
        }

        // Retrieve the window's specific rule configuration.
        let window_rule = WindowsApi::get_window_rule(handle);

        // Determine if border creation should proceed based on the window rule's enabled status.
        match window_rule.match_window.enabled {
            // If border creation is explicitly disabled, log and exit.
            Some(false) => {
                let window_title =
                    WindowsApi::get_process_name(handle).unwrap_or("unknown".to_string());
                info!(
                    "border creation is disabled for window: {} ({:?})",
                    window_title,
                    handle.as_hwnd()
                );
            }
            // If border creation is enabled or the rule doesn't specify, check for filtered styles.
            Some(true) | None if !WindowsApi::has_filtered_style(handle) => {
                // Create the border for the window using the retrieved rule.
                Border::create(handle, window_rule);
            }
            _ => {}
        }
    }

    pub fn hide(handle: isize) -> bool {
        let _ = std::thread::spawn(move || {
            if let Some(border) = window_border(handle) {
                WindowsApi::post_message_w(
                    Some(border.border_window.as_hwnd()),
                    WM_APP_HIDECLOAKED,
                    WPARAM(0),
                    LPARAM(0),
                )
                .context("border::hide")
                .log_if_err();
            }
        });
        true
    }

    pub fn create(tracking_window: isize, window_rule: WindowRule) {
        std::thread::spawn(move || {
            let mut borders_hashmap = window_borders();

            // Check to see if there is already a border for the given tracking window
            if borders_hashmap.contains_key(&tracking_window) {
                return;
            }

            let mut border = Self {
                tracking_window,
                window_rect: Rect(RECT::default()),
                ..Default::default()
            };

            if let Err(e) = border.create_border_window(&window_rule) {
                error!("could not create border window: {e:?}");
                return;
            };

            borders_hashmap.insert(tracking_window, border.clone());

            drop(borders_hashmap);
            let _ = window_rule;
            let _ = tracking_window;

            if let Err(e) = border.init() {
                error!("{e}");
            }
        });
    }

    pub fn create_border_window(&mut self, window_rule: &WindowRule) -> anyhow::Result<()> {
        let title = format!(
            "tacky-border | {} | {:?}",
            WindowsApi::get_window_title(self.tracking_window).unwrap_or_default(),
            self.tracking_window
        )
        .as_raw_pcwstr();

        self.border_window = WindowsApi::create_border_window(title, self)?;
        self.load_from_config(window_rule)?;
        self.process_name = WindowsApi::get_process_name(self.tracking_window)
            .unwrap_or_else(|_| "unknown".to_string());

        debug!(
            "[create_border_window] Border created for: Process - {} (Tracking Window ID: {:?})",
            self.process_name,
            self.tracking_window.as_hwnd()
        );

        Ok(())
    }

    pub fn init(&mut self) -> anyhow::Result<()> {
        thread::sleep(time::Duration::from_millis(self.initialize_delay as u64));

        unsafe {
            // Make the window border transparent
            let pos: i32 = -GetSystemMetrics(SM_CXVIRTUALSCREEN) - 8;
            let hrgn = CreateRectRgn(pos, 0, pos + 1, 1);
            let mut bh: DWM_BLURBEHIND = Default::default();
            if !hrgn.is_invalid() {
                bh = DWM_BLURBEHIND {
                    dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
                    fEnable: TRUE,
                    hRgnBlur: hrgn,
                    fTransitionOnMaximized: FALSE,
                };
            }

            DwmEnableBlurBehindWindow(self.border_window.as_hwnd(), &bh)
                .context("could not make window transparent")?;

            WindowsApi::set_layered_window_attributes(
                self.border_window,
                COLORREF(0),
                0,
                LWA_COLORKEY,
            )
            .context("could not set LWA_COLORKEY")?;

            WindowsApi::set_layered_window_attributes(
                self.border_window,
                COLORREF(-1i32 as u32),
                255,
                LWA_ALPHA,
            )
            .context("could not set LWA_ALPHA")?;

            self.render_resources
                .create(
                    self.current_monitor,
                    self.width,
                    self.window_padding,
                    self.border_window,
                )
                .context("could not create render resources in init()")?;

            let d2d_context = self.render_resources.d2d_context()?;
            let border_bitmap = self.render_resources.border_bitmap()?;
            let mask_bitmap = self.render_resources.mask_bitmap()?;

            // Initialize the command lists used to draw effects
            self.effect_manager
                .create_command_lists_if_enabled(d2d_context, border_bitmap, mask_bitmap)
                .context("could not create command list")?;

            let brush_properties = D2D1_BRUSH_PROPERTIES {
                opacity: 0.0,
                transform: Matrix3x2::identity(),
            };

            self.render_rect = D2D1_ROUNDED_RECT {
                rect: Default::default(),
                radiusX: self.radius,
                radiusY: self.radius,
            };

            self.active_color
                .to_d2d1_brush(d2d_context, &self.window_rect.into(), &brush_properties)
                .log_if_err();
            self.inactive_color
                .to_d2d1_brush(d2d_context, &self.window_rect.into(), &brush_properties)
                .log_if_err();

            self.update_color(Some(self.initialize_delay)).log_if_err();
            self.update_window_rect().log_if_err();

            if WindowsApi::has_native_border(self.tracking_window) {
                self.update_position(Some(SWP_SHOWWINDOW)).log_if_err();
                self.render().log_if_err();

                // Sometimes, it doesn't show the window at first, so we wait 5ms and update it.
                // This is very hacky and needs to be looked into. It may be related to the issue
                // detailed in the wnd_proc. TODO
                thread::sleep(time::Duration::from_millis(5));
                self.update_position(Some(SWP_SHOWWINDOW)).log_if_err();
                self.render().log_if_err();
            }

            self.animation_manager
                .set_timer(self.border_window)
                .log_if_err();

            if WindowsApi::is_window_minimized(self.tracking_window) {
                WindowsApi::post_message_w(
                    Some(self.border_window.as_hwnd()),
                    WM_APP_MINIMIZESTART,
                    WPARAM(0),
                    LPARAM(0),
                )
                .context("could not post WM_APP_MINIMIZESTART message in init()")
                .log_if_err();
            }

            self.render_resources
                .update(self.current_monitor, self.width, self.window_padding)
                .log_if_err();
            let d2d_context = self.render_resources.d2d_context()?;
            let border_bitmap = self.render_resources.border_bitmap()?;
            let mask_bitmap = self.render_resources.mask_bitmap()?;
            self.effect_manager
                .create_command_lists_if_enabled(d2d_context, border_bitmap, mask_bitmap)
                .context("could not create effects command list")?;

            debug!(
                "[init] Window Border Event: Started (Process: {}, Tracking Window ID: {:?})",
                self.process_name,
                self.tracking_window()
            );

            let mut message = MSG::default();

            loop {
                // Get the next message from the message queue
                if WindowsApi::get_message_w(&mut message, None, 0, 0).as_bool() {
                    // Translate and dispatch the message
                    let _ = WindowsApi::translate_message(&message);
                    WindowsApi::dispatch_message_w(&message);
                } else if message.message == WM_QUIT {
                    debug!(
                        "[init] Window Border Event: Stopping (Process: {}, Tracking Window ID: {:?})",
                        self.process_name,
                        self.tracking_window()
                    );
                    break;
                } else {
                    let last_error = GetLastError();
                    error!(
                        "[init] Window Border Event: Stopping (Process: {}, Tracking Window ID: {:?}) (error: {last_error:?})",
                        self.process_name,
                        self.tracking_window()
                    );
                    return Err(anyhow!("unexpected exit from message loop.".to_string()));
                }
            }

            debug!(
                "[init] Window Border Event: Stopped (Process: {}, Tracking Window ID: {:?})",
                self.process_name,
                self.tracking_window()
            );
        }

        Ok(())
    }

    fn load_from_config(&mut self, window_rule: &WindowRule) -> anyhow::Result<()> {
        let current_dpi = match WindowsApi::get_dpi_for_window(self.tracking_window) {
            Ok(dpi) => dpi as f32,
            Err(err) => {
                self.exit_border_thread();
                return Err(anyhow!("could not get dpi for window: {err}"));
            }
        };

        let user_config = AppManager::get().config().clone();
        let config = ParsedConfig::try_from(user_config)?;
        let global = &config.global_rule;

        let config_width = window_rule
            .match_window
            .border_width
            .unwrap_or(config.global_rule.border_width);
        let config_offset = window_rule
            .match_window
            .border_offset
            .unwrap_or(config.global_rule.border_offset);

        let active_color = window_rule
            .match_window
            .active_color
            .as_ref()
            .unwrap_or(&global.active_color);

        let inactive_color = window_rule
            .match_window
            .inactive_color
            .as_ref()
            .unwrap_or(&global.inactive_color);

        let animation_manager = window_rule
            .match_window
            .animation_manager
            .as_ref()
            .unwrap_or(&global.animation_manager);

        let effect_manager = window_rule
            .match_window
            .effect_manager
            .as_ref()
            .unwrap_or(&global.effect_manager);

        let config_style = window_rule
            .match_window
            .border_style
            .as_ref()
            .unwrap_or(&global.border_style);

        self.active_color = active_color.clone();
        self.inactive_color = inactive_color.clone();

        self.current_monitor = WindowsApi::monitor_from_window(self.tracking_window);
        self.current_dpi = current_dpi;

        self.width = (config_width as f32 * current_dpi / 96.0).round() as i32;
        self.radius = config_style.to_radius(self.width, current_dpi, self.tracking_window);
        self.offset = config_offset;

        self.animation_manager = animation_manager.clone();
        self.effect_manager = effect_manager.clone();

        let max_active_padding = self
            .effect_manager
            .active()
            .iter()
            .max_by_key(|params| {
                // Try to find the effect params with the largest required padding
                let max_std_dev = params.standard_deviation;
                let max_translation = (params.translation.x).max(params.translation.y);

                ((max_std_dev * 3.0).ceil() + max_translation.ceil()) as i32
            })
            .map(|params| {
                // Now that we found it, go ahead and calculate it as an f32
                let max_std_dev = params.standard_deviation;
                let max_translation = (params.translation.x).max(params.translation.y);

                (max_std_dev * 3.0).ceil() + max_translation.ceil()
            })
            .unwrap_or(0.0);
        let max_inactive_padding = self
            .effect_manager
            .inactive()
            .iter()
            .max_by_key(|params| {
                // Try to find the effect params with the largest required padding
                let max_std_dev = params.standard_deviation;
                let max_translation = (params.translation.x).max(params.translation.y);

                // 3 standard deviations gets us 99.7% coverage, which should be good enough
                ((max_std_dev * 3.0).ceil() + max_translation.ceil()) as i32
            })
            .map(|params| {
                // Now that we found it, go ahead and calculate it as an f32
                let max_std_dev = params.standard_deviation;
                let max_translation = (params.translation.x).max(params.translation.y);

                // 3 standard deviations gets us 99.7% coverage, which should be good enough
                (max_std_dev * 3.0).ceil() + max_translation.ceil()
            })
            .unwrap_or(0.0);

        self.window_padding = max_active_padding.max(max_inactive_padding).ceil() as i32;

        let available_windows = WindowsApi::collect_window_handles().unwrap_or_default();

        self.initialize_delay = match available_windows.contains(&self.tracking_window) {
            true => 0,
            false => window_rule
                .match_window
                .initialize_delay
                .unwrap_or(global.initialize_delay),
        };

        self.unminimize_delay = window_rule
            .match_window
            .unminimize_delay
            .unwrap_or(global.unminimize_delay);

        Ok(())
    }

    fn update_window_rect(&mut self) -> anyhow::Result<()> {
        self.window_rect = WindowsApi::window_rect(self.tracking_window).map_err(|e| {
            self.exit_border_thread(); // Exit the thread on error
            e.context(format!(
                "could not get window rect for: {:?}",
                self.tracking_window()
            )) // Add context
        })?;

        self.window_rect
            .add_margin(self.width + self.window_padding);

        Ok(())
    }

    fn update_position(&mut self, other_flags: Option<SET_WINDOW_POS_FLAGS>) -> anyhow::Result<()> {
        // Attempt to set the window position with the provided flags
        WindowsApi::set_border_pos(
            self.border_window,
            &self.window_rect,
            self.tracking_window,
            other_flags,
        )
        .with_context(|| {
            format!(
                "Failed to set position for window: {} ({:?})",
                WindowsApi::get_process_name(self.tracking_window)
                    .unwrap_or_else(|_| "unknown".to_string()),
                self.tracking_window
            )
        })
        .inspect_err(|_| {
            // Side-effect for error handling: Clean up on error
            self.exit_border_thread();
        })?;

        Ok(())
    }

    fn update_color(&mut self, check_delay: Option<u32>) -> anyhow::Result<()> {
        self.is_window_active = self.tracking_window == *get_active_window();

        if self.current_animations().contains_kind(AnimationKind::Fade) {
            if check_delay.is_some_and(|delay| delay == 0) {
                // More idiomatic check
                self.update_brush_opacities();
                self.refresh_fade_progress();
            } else {
                self.animation_manager.flags.should_fade = true;
            }
        } else {
            self.update_brush_opacities();
        }

        Ok(())
    }

    fn refresh_fade_progress(&mut self) {
        self.animation_manager.progress.fade = if self.is_window_active { 1.0 } else { 0.0 };
    }

    fn update_brush_opacities(&mut self) {
        let (top_color, bottom_color) = if self.is_window_active {
            (&mut self.active_color, &mut self.inactive_color)
        } else {
            (&mut self.inactive_color, &mut self.active_color)
        };
        top_color.set_opacity(1.0);
        bottom_color.set_opacity(0.0);
    }

    fn update_width_radius(&mut self) {
        let window_rule = WindowsApi::get_window_rule(self.tracking_window);
        let config = AppManager::get().config().clone();
        let global = &config.global_rule;

        let width_config = window_rule
            .match_window
            .border_width
            .unwrap_or(global.border_width);
        let style_config = window_rule
            .match_window
            .border_style
            .as_ref()
            .unwrap_or(&global.border_style);

        self.width = (width_config as f32 * self.current_dpi / 96.0).round() as i32;
        self.radius = style_config.to_radius(self.width, self.current_dpi, self.tracking_window);
    }

    fn current_animations(&self) -> &AnimationEngineVec {
        if self.is_window_active {
            self.animation_manager.get_active_animation()
        } else {
            self.animation_manager.get_inactive_animation()
        }
    }

    fn current_effects(&self) -> &EffectEngineVec {
        if self.is_window_active {
            self.effect_manager.active()
        } else {
            self.effect_manager.inactive()
        }
    }

    fn current_command_list(&self) -> anyhow::Result<&ID2D1CommandList> {
        if self.is_window_active {
            self.effect_manager.active_command_list()
        } else {
            self.effect_manager.inactive_command_list()
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.last_render_time = Some(std::time::Instant::now());
        let rect_width = self.window_rect.width() as f32;
        let rect_height = self.window_rect.height() as f32;

        let border_width = self.width as f32;
        let border_offset = self.offset as f32;
        let window_padding = self.window_padding as f32;
        let border_radius = self.radius;

        self.render_rect.rect = D2D_RECT_F {
            left: border_width / 2.0 + window_padding - border_offset,
            top: border_width / 2.0 + window_padding - border_offset,
            right: rect_width - border_width / 2.0 - window_padding + border_offset,
            bottom: rect_height - border_width / 2.0 - window_padding + border_offset,
        };

        if !self.current_effects().is_empty() {
            self.render_with_effects(border_radius)?;
            return Ok(());
        }

        let d2d_context = self.render_resources.d2d_context()?;

        unsafe {
            let (bottom_color, top_color) = match self.is_window_active {
                true => (&self.inactive_color, &self.active_color),
                false => (&self.active_color, &self.inactive_color),
            };

            let target_bitmap = self.render_resources.target_bitmap()?;
            d2d_context.SetTarget(target_bitmap);

            d2d_context.BeginDraw();
            d2d_context.Clear(None);

            if bottom_color.get_opacity() > Some(0.0) {
                if let Color::Gradient(gradient) = bottom_color {
                    gradient.update_start_end_points(&self.window_rect.into());
                }

                match bottom_color.get_brush() {
                    Some(id2d1_brush) => {
                        self.draw_rectangle(d2d_context, id2d1_brush, border_radius)
                    }
                    None => debug!("ID2D1Brush for bottom_color has not been created yet"),
                }
            }

            if top_color.get_opacity() > Some(0.0) {
                if let Color::Gradient(gradient) = top_color {
                    gradient.update_start_end_points(&self.window_rect.into());
                }

                match top_color.get_brush() {
                    Some(id2d1_brush) => {
                        self.draw_rectangle(d2d_context, id2d1_brush, border_radius)
                    }
                    None => debug!("ID2D1Brush for top_color has not been created yet"),
                }
            }

            if let Err(err) = d2d_context.EndDraw(None, None) {
                self.handle_end_draw_error(err.clone());
                return Err(err.into());
            }

            // Present the swap chain buffer
            let hresult = self
                .render_resources
                .swap_chain()?
                .Present(1, DXGI_PRESENT::default());
            if hresult != S_OK {
                return Err(anyhow!("could not present swap_chain: {hresult}"));
            }
        }

        Ok(())
    }

    fn render_with_effects(&mut self, border_radius: f32) -> anyhow::Result<()> {
        let d2d_context = self.render_resources.d2d_context()?;

        unsafe {
            let (bottom_color, top_color) = match self.is_window_active {
                true => (&self.inactive_color, &self.active_color),
                false => (&self.active_color, &self.inactive_color),
            };

            let render_rect_adjusted = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: self.render_rect.rect.left - (self.width as f32 / 2.0),
                    top: self.render_rect.rect.top - (self.width as f32 / 2.0),
                    right: self.render_rect.rect.right + (self.width as f32 / 2.0),
                    bottom: self.render_rect.rect.bottom + (self.width as f32 / 2.0),
                },
                radiusX: border_radius + (self.width as f32 / 2.0),
                radiusY: border_radius + (self.width as f32 / 2.0),
            };

            let border_bitmap = self.render_resources.border_bitmap()?;
            d2d_context.SetTarget(border_bitmap);

            d2d_context.BeginDraw();
            d2d_context.Clear(None);

            if bottom_color.get_opacity() > Some(0.0) {
                if let Color::Gradient(gradient) = bottom_color {
                    gradient.update_start_end_points(&self.window_rect.into());
                }

                match bottom_color.get_brush() {
                    Some(id2d1_brush) => self.fill_rectangle(
                        &render_rect_adjusted,
                        d2d_context,
                        id2d1_brush,
                        border_radius,
                    ),
                    None => debug!("ID2D1Brush for bottom_color has not been created yet"),
                }
            }

            if top_color.get_opacity() > Some(0.0) {
                if let Color::Gradient(gradient) = top_color {
                    gradient.update_start_end_points(&self.window_rect.into());
                }

                match top_color.get_brush() {
                    Some(id2d1_brush) => self.fill_rectangle(
                        &render_rect_adjusted,
                        d2d_context,
                        id2d1_brush,
                        border_radius,
                    ),
                    None => debug!("ID2D1Brush for top_color has not been created yet"),
                }
            }

            if let Err(err) = d2d_context.EndDraw(None, None) {
                self.handle_end_draw_error(err.clone());
                return Err(err.into());
            }

            // Set the d2d_context target to the mask_bitmap so we can create an alpha mask
            let mask_bitmap = self.render_resources.mask_bitmap()?;
            d2d_context.SetTarget(mask_bitmap);

            // Create our mask geometry (masks out inner glow/blur)
            let render_rect_adjusted = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: self.render_rect.rect.left + (self.width as f32 / 2.0),
                    top: self.render_rect.rect.top + (self.width as f32 / 2.0),
                    right: self.render_rect.rect.right - (self.width as f32 / 2.0),
                    bottom: self.render_rect.rect.bottom - (self.width as f32 / 2.0),
                },
                radiusX: border_radius - (self.width as f32 / 2.0),
                radiusY: border_radius - (self.width as f32 / 2.0),
            };

            // Create a 100% opaque brush because our active/inactive colors' brushes might not be
            let opaque_brush = d2d_context
                .CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                    None,
                )
                .context("opaque_brush")?;

            // Draw to the mask_bitmap
            d2d_context.BeginDraw();
            d2d_context.Clear(None);

            self.fill_rectangle(
                &render_rect_adjusted,
                d2d_context,
                &opaque_brush,
                border_radius,
            );

            if let Err(err) = d2d_context.EndDraw(None, None) {
                self.handle_end_draw_error(err.clone());
                return Err(err.into());
            }

            // Set d2d_context's target back to the target_bitmap so we can draw to the display
            let target_bitmap = self.render_resources.target_bitmap()?;
            d2d_context.SetTarget(target_bitmap);

            // Retrieve our command list (includes border_bitmap, mask_bitmap, and effects)
            let command_list = self.current_command_list()?;

            // Draw to the target_bitmap
            d2d_context.BeginDraw();
            d2d_context.Clear(None);

            // Draw using the command list
            d2d_context.DrawImage(
                command_list,
                None,
                None,
                D2D1_INTERPOLATION_MODE_LINEAR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            );

            if let Err(err) = d2d_context.EndDraw(None, None) {
                self.handle_end_draw_error(err.clone());
                return Err(err.into());
            }

            // Present the swap chain buffer
            let hresult = self
                .render_resources
                .swap_chain()?
                .Present(1, DXGI_PRESENT::default());
            if hresult != S_OK {
                return Err(anyhow!("could not present swap_chain: {hresult}"));
            }
        }

        Ok(())
    }

    fn draw_rectangle(
        &self,
        d2d_context: &ID2D1DeviceContext7,
        brush: &ID2D1Brush,
        border_radius: f32,
    ) {
        unsafe {
            match border_radius {
                0.0 => d2d_context.DrawRectangle(
                    &self.render_rect.rect,
                    brush,
                    self.width as f32,
                    None,
                ),
                _ => d2d_context.DrawRoundedRectangle(
                    &self.render_rect,
                    brush,
                    self.width as f32,
                    None,
                ),
            }
        }
    }

    fn fill_rectangle(
        &self,
        rounded_rect: &D2D1_ROUNDED_RECT,
        d2d_context: &ID2D1DeviceContext7,
        brush: &ID2D1Brush,
        border_radius: f32,
    ) {
        unsafe {
            match border_radius {
                0.0 => d2d_context.FillRectangle(&rounded_rect.rect, brush),
                _ => d2d_context.FillRoundedRectangle(rounded_rect, brush),
            }
        }
    }

    fn handle_end_draw_error(&mut self, err: windows::core::Error) {
        if err.code() == D2DERR_RECREATE_TARGET {
            // D2DERR_RECREATE_TARGET is recoverable if we just recreate the render target.
            // This error can be caused by things like waking up from sleep, updating GPU
            // drivers, changing screen resolution, etc.
            warn!("render target has been lost; attempting to recreate");

            if let Err(err_2) = self.render_resources.create(
                self.current_monitor,
                self.width,
                self.window_padding,
                self.border_window,
            ) {
                error!("could not recreate render target; exiting thread: {err_2}");
                self.exit_border_thread();
                return;
            }

            // This really should not fail. If it does, I messed up somewhere.
            let (Ok(d2d_context), Ok(border_bitmap), Ok(mask_bitmap)) = (
                self.render_resources.d2d_context(),
                self.render_resources.border_bitmap(),
                self.render_resources.mask_bitmap(),
            ) else {
                error!("could not get render resources even after recreating them; exiting thread");
                self.exit_border_thread();
                return;
            };

            if let Err(err_3) = self.effect_manager.create_command_lists_if_enabled(
                d2d_context,
                border_bitmap,
                mask_bitmap,
            ) {
                error!("could not recreate effects command lists; exiting thread: {err_3}");
                self.exit_border_thread();
                return;
            }

            info!("successfully recreated render target; resuming thread");
        } else {
            error!("d2d_context.EndDraw() failed; exiting thread: {err}");
            self.exit_border_thread();
        }
    }

    fn exit_border_thread(&mut self) {
        self.pause = true;
        self.animation_manager
            .kill_timer(self.border_window)
            .log_if_err();
        let mut borders_hashmap = window_borders();
        borders_hashmap.remove(&(self.tracking_window));

        drop(borders_hashmap);
        WindowsApi::post_quit_message(0);
    }

    pub fn destroy(&self) {
        WindowsApi::destroy_window(self.border_window)
            .context("destroy_border_for_window")
            .log_if_err();
    }

    fn callback(&mut self, window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match message {
            // EVENT_OBJECT_LOCATIONCHANGE
            WM_APP_LOCATIONCHANGE => {
                if self.pause {
                    return LRESULT(0);
                }

                let mut should_render = false;

                if !WindowsApi::has_native_border(self.tracking_window) {
                    self.update_position(Some(SWP_HIDEWINDOW)).log_if_err();
                    return LRESULT(0);
                }

                let old_rect = self.window_rect;
                self.update_window_rect().log_if_err();

                if !self.window_rect.is_visible() {
                    self.window_rect = old_rect;
                    return LRESULT(0);
                }

                if !self.window_rect.is_same_size_as(&old_rect) {
                    should_render |= true;
                }

                let update_pos_flags =
                    (!WindowsApi::is_window_visible(self.border_window)).then_some(SWP_SHOWWINDOW);
                self.update_position(update_pos_flags).log_if_err();

                let new_monitor = WindowsApi::monitor_from_window(self.tracking_window);

                if new_monitor != self.current_monitor {
                    self.current_monitor = new_monitor;
                    self.render_resources
                        .update(self.current_monitor, self.width, self.window_padding)
                        .context("could not update render resources")
                        .log_if_err();

                    if let (Ok(d2d_context), Ok(border_bitmap), Ok(mask_bitmap)) = (
                        self.render_resources.d2d_context(),
                        self.render_resources.border_bitmap(),
                        self.render_resources.mask_bitmap(),
                    ) {
                        self.effect_manager
                            .create_command_lists_if_enabled(
                                d2d_context,
                                border_bitmap,
                                mask_bitmap,
                            )
                            .log_if_err();
                    } else {
                        error!("could not get resources to create effects command list");
                    }

                    let new_dpi = match WindowsApi::get_dpi_for_window(self.tracking_window) {
                        Ok(dpi) => dpi as f32,
                        Err(err) => {
                            error!("could not get dpi for window: {err}");
                            self.exit_border_thread();
                            return LRESULT(0);
                        }
                    };

                    if new_dpi != self.current_dpi {
                        self.current_dpi = new_dpi;
                        self.update_width_radius();
                    }

                    should_render |= true;
                }

                if should_render {
                    self.render().log_if_err();
                }
            }
            // EVENT_OBJECT_REORDER
            WM_APP_REORDER => {
                // For apps like firefox, when you hover over a tab, a popup window spawns that
                // changes the z-order and causes the border to sit under the tracking window. To
                // remedy that, we just re-update the position/z-order when windows are reordered.
                self.update_position(None).log_if_err();
            }
            // EVENT_SYSTEM_FOREGROUND
            WM_APP_FOREGROUND => {
                self.update_color(None).log_if_err();
                self.update_position(None).log_if_err();
                self.render().log_if_err();
            }
            // EVENT_OBJECT_SHOW / EVENT_OBJECT_UNCLOAKED
            WM_APP_SHOWUNCLOAKED => {
                // With GlazeWM, if I switch to another workspace while a window is minimized and
                // switch back, then we will receive this message even though the window is not yet
                // visible. And, the window rect will be all weird. So, we apply the following fix.
                let old_rect = self.window_rect;
                self.update_window_rect().log_if_err();

                if !self.window_rect.is_visible() {
                    self.window_rect = old_rect;
                    return LRESULT(0);
                }

                if WindowsApi::has_native_border(self.tracking_window) {
                    self.update_position(Some(SWP_SHOWWINDOW)).log_if_err();
                    self.render().log_if_err();
                }

                self.animation_manager
                    .set_timer(self.border_window)
                    .log_if_err();

                self.pause = false;
            }
            // EVENT_OBJECT_HIDE / EVENT_OBJECT_CLOAKED
            WM_APP_HIDECLOAKED => {
                self.update_position(Some(SWP_HIDEWINDOW)).log_if_err();
                self.animation_manager
                    .kill_timer(self.border_window)
                    .log_if_err();
                self.pause = true;
            }
            // EVENT_OBJECT_MINIMIZESTART
            WM_APP_MINIMIZESTART => {
                self.update_position(Some(SWP_HIDEWINDOW)).log_if_err();

                self.active_color.set_opacity(0.0);
                self.inactive_color.set_opacity(0.0);

                self.animation_manager
                    .kill_timer(self.border_window)
                    .log_if_err();

                self.pause = true;
            }
            // EVENT_SYSTEM_MINIMIZEEND
            // When a window is about to be unminimized, hide the border and let the thread sleep
            // to wait for the window animation to finish, then show the border.
            WM_APP_MINIMIZEEND => {
                thread::sleep(time::Duration::from_millis(self.unminimize_delay as u64));

                self.animation_manager.set_last_animation_time(None);

                if WindowsApi::has_native_border(self.tracking_window) {
                    self.update_color(Some(self.unminimize_delay)).log_if_err();
                    self.update_window_rect().log_if_err();
                    self.update_position(Some(SWP_SHOWWINDOW)).log_if_err();
                    self.render().log_if_err();
                }

                self.animation_manager
                    .set_timer(self.border_window)
                    .log_if_err();

                self.pause = false;
            }
            WM_APP_TIMER => {
                if self.pause {
                    return LRESULT(0);
                }

                let animation_elapsed = self.animation_manager.last_animation_time().elapsed();
                let render_elapsed = self
                    .last_render_time
                    .unwrap_or(time::Instant::now())
                    .elapsed();

                self.animation_manager.set_last_animation_time(None);

                let mut animations_updated = false;

                let current_animations = self.current_animations();

                if current_animations.clone().is_empty() {
                    self.active_color.set_transform(&Matrix3x2::identity());
                    self.inactive_color.set_transform(&Matrix3x2::identity());
                    animations_updated = false;
                } else {
                    for animation in current_animations.clone() {
                        match animation.kind {
                            AnimationKind::Spiral | AnimationKind::ReverseSpiral => {
                                animation.play(self, &animation_elapsed);
                                animations_updated = true;
                            }
                            AnimationKind::Fade => {
                                if self.animation_manager.flags.should_fade {
                                    animation.play(self, &animation_elapsed);
                                    animations_updated = true;
                                }
                            }
                        }
                    }
                }

                let interval = 1.0 / self.animation_manager.fps();
                let diff = render_elapsed.as_secs_f32() - interval;
                if animations_updated && (diff.abs() <= 0.001 || diff >= 0.0) {
                    self.render().log_if_err();
                }
            }
            WM_PAINT => {
                let _ = WindowsApi::validate_rect(Some(window.as_int()), None);
            }
            WM_NCDESTROY => {
                WindowsApi::set_window_long_ptr_w(window.as_int(), GWLP_USERDATA, 0);
                self.exit_border_thread();
            }
            // Ignore these window position messages
            WM_WINDOWPOSCHANGING | WM_WINDOWPOSCHANGED => {}
            _ => {
                return WindowsApi::def_window_proc_w(window.as_int(), message, wparam.0, lparam.0);
            }
        }
        LRESULT(0)
    }

    pub extern "system" fn wnd_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let mut border_pointer: *mut Border =
            WindowsApi::window_long_ptr_w(window.as_int(), GWLP_USERDATA) as _;

        if border_pointer.is_null() && message == WM_CREATE {
            let create_struct: *mut CREATESTRUCTW = lparam.0 as *mut _;
            border_pointer = unsafe { (*create_struct).lpCreateParams as *mut _ };
            WindowsApi::set_window_long_ptr_w(window.as_int(), GWLP_USERDATA, border_pointer as _);
        }

        match !border_pointer.is_null() {
            true => unsafe { (*border_pointer).callback(window, message, wparam, lparam) },
            false => WindowsApi::def_window_proc_w(window.as_int(), message, wparam.0, lparam.0),
        }
    }
}
