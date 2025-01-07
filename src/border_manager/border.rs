use crate::animation::manager::AnimationManager;
use crate::animation::wrapper::AnimationEngineVec;
use crate::as_ptr;
use crate::core::animation::AnimationKind;
use crate::core::app_state::APP_STATE;
use crate::core::rect::Rect;
use crate::error::LogIfErr;
use crate::user_config::BorderStyle;
use crate::user_config::WindowRuleConfig;
use crate::windows_api::ToWideString;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_FOREGROUND;
use crate::windows_api::WM_APP_HIDECLOAKED;
use crate::windows_api::WM_APP_LOCATIONCHANGE;
use crate::windows_api::WM_APP_MINIMIZEEND;
use crate::windows_api::WM_APP_MINIMIZESTART;
use crate::windows_api::WM_APP_REORDER;
use crate::windows_api::WM_APP_SHOWUNCLOAKED;
use crate::windows_api::WM_APP_TIMER;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result as AnyResult;
use std::thread;
use std::time;
use std::time::Instant;
use win_color::Color;
use win_color::ColorImpl;
use win_color::GlobalColorImpl;
use win_color::GradientImpl;
use windows::core::CloneType;
use windows::core::TypeKind;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::D2DERR_RECREATE_TARGET;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::RECT;
use windows::Win32::Foundation::TRUE;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::Graphics::Direct2D::Common::D2D1_ALPHA_MODE_PREMULTIPLIED;
use windows::Win32::Graphics::Direct2D::Common::D2D1_PIXEL_FORMAT;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U;
use windows::Win32::Graphics::Direct2D::ID2D1Brush;
use windows::Win32::Graphics::Direct2D::ID2D1HwndRenderTarget;
use windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_PER_PRIMITIVE;
use windows::Win32::Graphics::Direct2D::D2D1_BRUSH_PROPERTIES;
use windows::Win32::Graphics::Direct2D::D2D1_HWND_RENDER_TARGET_PROPERTIES;
use windows::Win32::Graphics::Direct2D::D2D1_PRESENT_OPTIONS_IMMEDIATELY;
use windows::Win32::Graphics::Direct2D::D2D1_PRESENT_OPTIONS_RETAIN_CONTENTS;
use windows::Win32::Graphics::Direct2D::D2D1_RENDER_TARGET_PROPERTIES;
use windows::Win32::Graphics::Direct2D::D2D1_RENDER_TARGET_TYPE_DEFAULT;
use windows::Win32::Graphics::Direct2D::D2D1_ROUNDED_RECT;
use windows::Win32::Graphics::Dwm::DwmEnableBlurBehindWindow;
use windows::Win32::Graphics::Dwm::DWMWA_EXTENDED_FRAME_BOUNDS;
use windows::Win32::Graphics::Dwm::DWM_BB_BLURREGION;
use windows::Win32::Graphics::Dwm::DWM_BB_ENABLE;
use windows::Win32::Graphics::Dwm::DWM_BLURBEHIND;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;
use windows::Win32::Graphics::Gdi::CreateRectRgn;
use windows::Win32::Graphics::Gdi::ValidateRect;
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
use windows::Win32::UI::WindowsAndMessaging::GetWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::SetWindowPos;
use windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
use windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;
use windows::Win32::UI::WindowsAndMessaging::GW_HWNDPREV;
use windows::Win32::UI::WindowsAndMessaging::HWND_TOP;
use windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA;
use windows::Win32::UI::WindowsAndMessaging::LWA_COLORKEY;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS;
use windows::Win32::UI::WindowsAndMessaging::SM_CXVIRTUALSCREEN;
use windows::Win32::UI::WindowsAndMessaging::SWP_HIDEWINDOW;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOREDRAW;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOSENDCHANGING;
use windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER;
use windows::Win32::UI::WindowsAndMessaging::SWP_SHOWWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WM_CREATE;
use windows::Win32::UI::WindowsAndMessaging::WM_NCDESTROY;
use windows::Win32::UI::WindowsAndMessaging::WM_PAINT;
use windows::Win32::UI::WindowsAndMessaging::WM_QUIT;
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGED;
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGING;

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
    pub width: i32,
    pub offset: i32,
    pub style: BorderStyle,
    pub render_target: Option<ID2D1HwndRenderTarget>,
    pub rounded_rect: D2D1_ROUNDED_RECT,
    pub active_color: Color,
    pub inactive_color: Color,
    pub animation_manager: AnimationManager,
    pub last_render_time: Option<Instant>,
    pub initialize_delay: u64,
    pub unminimize_delay: u64,
    pub pause: bool,
    pub current_dpi: f32,
}

impl Border {
    pub const fn border_window(&self) -> HWND {
        HWND(as_ptr!(self.border_window))
    }

    pub const fn tracking_window(&self) -> HWND {
        HWND(as_ptr!(self.tracking_window))
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
                    HWND(as_ptr!(handle))
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
                HWND(as_ptr!(existing_border.border_window)),
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
                info!(
                    "border creation is disabled for window: {:?}",
                    HWND(as_ptr!(handle))
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
                    HWND(as_ptr!(border.border_window)),
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

    pub fn create(tracking_window: isize, window_rule: WindowRuleConfig) {
        debug!("creating border for: {:?}", HWND(as_ptr!(tracking_window)));

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

    pub fn create_border_window(&mut self, window_rule: &WindowRuleConfig) -> AnyResult<()> {
        let title = format!(
            "tacky-border | {} | {:?}",
            WindowsApi::get_window_title(self.tracking_window).unwrap_or_default(),
            self.tracking_window
        )
        .as_raw_pcwstr();

        self.border_window = WindowsApi::create_border_window(title, self)?;
        self.load_from_config(window_rule)?;

        Ok(())
    }

    pub fn init(&mut self) -> AnyResult<()> {
        thread::sleep(time::Duration::from_millis(self.initialize_delay));

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

            DwmEnableBlurBehindWindow(HWND(as_ptr!(self.border_window)), &bh)
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

            if let Err(e) = self.create_render_resources() {
                error!("could not create render target in init(): {e:?}");
            }

            self.is_window_active = WindowsApi::is_window_active(self.tracking_window);

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
                    HWND(as_ptr!(self.border_window)),
                    WM_APP_MINIMIZESTART,
                    WPARAM(0),
                    LPARAM(0),
                )
                .context("could not post WM_APP_MINIMIZESTART message in init()")
                .log_if_err();
            }

            debug!("border window event started");

            let mut message = MSG::default();
            loop {
                // Get the next message from the message queue
                if WindowsApi::get_message_w(&mut message, None, 0, 0).as_bool() {
                    // Translate and dispatch the message
                    let _ = WindowsApi::translate_message(&message);
                    WindowsApi::dispatch_message_w(&message);
                } else if message.message == WM_QUIT {
                    debug!("border window event shutdown");
                    break;
                } else {
                    let last_error = GetLastError();
                    error!("border window event shutdown: {last_error:?}");
                    return Err(anyhow!("unexpected exit from message loop.".to_string()));
                }
            }

            debug!(
                "exiting border thread for {:?}!",
                HWND(as_ptr!(self.tracking_window))
            );
        }

        Ok(())
    }

    fn load_from_config(&mut self, window_rule: &WindowRuleConfig) -> AnyResult<()> {
        let config = (*APP_STATE.config.read().unwrap()).clone();
        let global = &config.global_rule;

        let config_width = window_rule
            .match_window
            .border_width
            .unwrap_or(config.global_rule.border_width);
        let config_offset = window_rule
            .match_window
            .border_offset
            .unwrap_or(config.global_rule.border_offset);

        let config_active = window_rule
            .match_window
            .active_color
            .as_ref()
            .unwrap_or(&global.active_color);

        let config_inactive = window_rule
            .match_window
            .inactive_color
            .as_ref()
            .unwrap_or(&global.inactive_color);

        let animations_config = window_rule
            .match_window
            .animations
            .as_ref()
            .unwrap_or(&global.animations);

        let config_style = window_rule
            .match_window
            .border_style
            .as_ref()
            .unwrap_or(&global.border_style);

        self.active_color = config_active.to_color()?;
        self.inactive_color = config_inactive.to_color()?;

        self.current_dpi = match WindowsApi::get_dpi_for_window(self.tracking_window) as f32 {
            0.0 => {
                self.exit_border_thread();
                return Err(anyhow!("received invalid dpi of 0 from GetDpiForWindow"));
            }
            valid_dpi => valid_dpi,
        };

        self.width = (config_width as f32 * self.current_dpi / 96.0).round() as i32;
        self.style = config_style.clone();
        // self.border_radius =
        //     config_radius.to_radius(self.border_width, self.current_dpi, self.tracking_window);
        self.offset = config_offset;

        self.animation_manager = AnimationManager::try_from(animations_config.clone())?;

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

    fn create_render_resources(&mut self) -> AnyResult<()> {
        let render_target_properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_UNKNOWN,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            ..Default::default()
        };
        let hwnd_render_target_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: self.border_window(),
            pixelSize: Default::default(),
            presentOptions: D2D1_PRESENT_OPTIONS_RETAIN_CONTENTS | D2D1_PRESENT_OPTIONS_IMMEDIATELY,
        };

        let brush_properties = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,
            transform: Matrix3x2::identity(),
        };

        let border_radius =
            self.style
                .to_radius(self.width, self.current_dpi, self.tracking_window);

        self.rounded_rect = D2D1_ROUNDED_RECT {
            rect: Default::default(),
            radiusX: border_radius,
            radiusY: border_radius,
        };

        // Initialize the actual border color assuming it is in focus
        unsafe {
            let render_target = APP_STATE.render_factory.CreateHwndRenderTarget(
                &render_target_properties,
                &hwnd_render_target_properties,
            )?;

            render_target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);

            self.active_color
                .to_d2d1_brush(&render_target, &self.window_rect.into(), &brush_properties)
                .log_if_err();
            self.inactive_color
                .to_d2d1_brush(&render_target, &self.window_rect.into(), &brush_properties)
                .log_if_err();

            self.render_target = Some(render_target);
        }

        Ok(())
    }

    fn update_window_rect(&mut self) -> AnyResult<()> {
        if let Err(e) = WindowsApi::dwm_get_window_attribute::<RECT>(
            self.tracking_window,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut self.window_rect,
        )
        .context(format!(
            "could not get window rect for {:?}",
            self.tracking_window
        )) {
            self.exit_border_thread();

            return Err(e);
        }

        self.window_rect.add_margin(self.width);

        Ok(())
    }

    fn update_position(&mut self, other_flags: Option<SET_WINDOW_POS_FLAGS>) -> AnyResult<()> {
        unsafe {
            // Place the window border above the tracking window
            let hwnd_above_tracking = GetWindow(self.tracking_window(), GW_HWNDPREV);

            let mut swp_flags = SWP_NOSENDCHANGING
                | SWP_NOACTIVATE
                | SWP_NOREDRAW
                | other_flags.unwrap_or_default();

            if hwnd_above_tracking == Ok(self.border_window()) {
                swp_flags |= SWP_NOZORDER;
            }

            if let Err(e) = SetWindowPos(
                self.border_window(),
                hwnd_above_tracking.unwrap_or(HWND_TOP),
                self.window_rect.left,
                self.window_rect.top,
                self.window_rect.width(),
                self.window_rect.height(),
                swp_flags,
            )
            .context(format!(
                "could not set window position for {:?}",
                self.tracking_window
            )) {
                self.exit_border_thread();
                return Err(e);
            }
        }
        Ok(())
    }

    fn update_color(&mut self, check_delay: Option<u64>) -> AnyResult<()> {
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
        let config = (*APP_STATE.config.read().unwrap()).clone();
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
        self.style = style_config.clone();
    }

    fn current_animations(&self) -> &AnimationEngineVec {
        if self.is_window_active {
            self.animation_manager.get_active_animation()
        } else {
            self.animation_manager.get_inactive_animation()
        }
    }

    fn render(&mut self) -> AnyResult<()> {
        self.last_render_time = Some(std::time::Instant::now());

        let Some(ref render_target) = self.render_target else {
            return Err(anyhow!("render_target has not been set yet"));
        };

        let rect_width = self.window_rect.width() as f32;
        let rect_height = self.window_rect.height() as f32;

        let border_width = self.width as f32;
        let border_offset = self.offset as f32;

        self.rounded_rect.rect = D2D_RECT_F {
            left: border_width / 2.0 - border_offset,
            top: border_width / 2.0 - border_offset,
            right: rect_width - border_width / 2.0 + border_offset,
            bottom: rect_height - border_width / 2.0 + border_offset,
        };

        unsafe {
            render_target
                .Resize(&D2D_SIZE_U {
                    width: rect_width as u32,
                    height: rect_height as u32,
                })
                .context("could not resize render_target")?;

            let (bottom_color, top_color) = match self.is_window_active {
                true => (&self.inactive_color, &self.active_color),
                false => (&self.active_color, &self.inactive_color),
            };

            render_target.BeginDraw();
            render_target.Clear(None);

            if bottom_color.get_opacity() > Some(0.0) {
                if let Color::Gradient(gradient) = bottom_color {
                    gradient.update_start_end_points(&self.window_rect.into());
                }

                match bottom_color.get_brush() {
                    Some(id2d1_brush) => self.draw_rectangle(render_target, id2d1_brush),
                    None => debug!("ID2D1Brush for bottom_color has not been created yet"),
                }
            }

            if top_color.get_opacity() > Some(0.0) {
                if let Color::Gradient(gradient) = top_color {
                    gradient.update_start_end_points(&self.window_rect.into());
                }

                match top_color.get_brush() {
                    Some(id2d1_brush) => self.draw_rectangle(render_target, id2d1_brush),
                    None => debug!("ID2D1Brush for top_color has not been created yet"),
                }
            }

            match render_target.EndDraw(None, None) {
                Ok(_) => {}
                Err(e) if e.code() == D2DERR_RECREATE_TARGET => {
                    // D2DERR_RECREATE_TARGET is recoverable if we just recreate the render target.
                    // This error can be caused by things like waking up from sleep, updating GPU
                    // drivers, screen resolution changing, etc.
                    warn!("render_target has been lost; attempting to recreate");

                    match self.create_render_resources() {
                        Ok(_) => info!("successfully recreated render_target; resuming thread"),
                        Err(e_2) => {
                            error!("could not recreate render_target; exiting thread: {e_2}");
                            self.exit_border_thread();
                        }
                    }
                }
                Err(other) => {
                    error!("render_target.EndDraw() failed; exiting thread: {other}");
                    self.exit_border_thread();
                }
            }
        }

        Ok(())
    }

    fn draw_rectangle(&self, render_target: &ID2D1HwndRenderTarget, brush: &ID2D1Brush) {
        let border_radius =
            self.style
                .to_radius(self.width, self.current_dpi, self.tracking_window);
        unsafe {
            match border_radius {
                0.0 => render_target.DrawRectangle(
                    &self.rounded_rect.rect,
                    brush,
                    self.width as f32,
                    None,
                ),
                _ => render_target.DrawRoundedRectangle(
                    &self.rounded_rect,
                    brush,
                    self.width as f32,
                    None,
                ),
            }
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

                let new_dpi = match WindowsApi::get_dpi_for_window(self.tracking_window) as f32 {
                    0.0 => {
                        error!("received invalid dpi of 0 from GetDpiForWindow");
                        self.exit_border_thread();
                        return LRESULT(0);
                    }
                    valid_dpi => valid_dpi,
                };

                if new_dpi != self.current_dpi {
                    self.current_dpi = new_dpi;
                    self.update_width_radius();
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
                thread::sleep(time::Duration::from_millis(self.unminimize_delay));

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

                // println!("time since last anim: {}", render_elapsed.as_secs_f32());

                let interval = 1.0 / self.animation_manager.fps();
                let diff = render_elapsed.as_secs_f32() - interval;
                if animations_updated && (diff.abs() <= 0.001 || diff >= 0.0) {
                    self.render().log_if_err();
                }
            }
            WM_PAINT => {
                let _ = unsafe { ValidateRect(window, None) };
            }
            WM_NCDESTROY => {
                unsafe { SetWindowLongPtrW(window, GWLP_USERDATA, 0) };
                self.exit_border_thread();
            }
            // Ignore these window position messages
            WM_WINDOWPOSCHANGING | WM_WINDOWPOSCHANGED => {}
            _ => {
                return unsafe { DefWindowProcW(window, message, wparam, lparam) };
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
        unsafe {
            let mut border_pointer: *mut Border = GetWindowLongPtrW(window, GWLP_USERDATA) as _;

            if border_pointer.is_null() && message == WM_CREATE {
                let create_struct: *mut CREATESTRUCTW = lparam.0 as *mut _;
                border_pointer = (*create_struct).lpCreateParams as *mut _;
                SetWindowLongPtrW(window, GWLP_USERDATA, border_pointer as _);
            }

            match !border_pointer.is_null() {
                true => (*border_pointer).callback(window, message, wparam, lparam),
                false => DefWindowProcW(window, message, wparam, lparam),
            }
        }
    }
}
