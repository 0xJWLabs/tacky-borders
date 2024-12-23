use crate::animations::animation::AnimationParameters;
use crate::animations::animation::AnimationType;
use crate::animations::timer::KillAnimationTimer;
use crate::animations::timer::SetAnimationTimer;
use crate::animations::Animations;
use crate::border_config::WindowRule;
use crate::border_config::CONFIG;
use crate::error::LogIfErr;
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
use rustc_hash::FxHashMap;
use std::ptr;
use std::sync::LazyLock;
use std::thread;
use std::time;
use std::time::Instant;
use win_color::Color;
use win_color::ColorImpl;
use win_color::GlobalColorImpl;
use win_color::GradientImpl;
use windows::core::w;
use windows::core::CloneType;
use windows::core::Result as WinResult;
use windows::core::TypeKind;
use windows::core::PCWSTR;
use windows::Foundation::Numerics::Matrix3x2;
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
use windows::Win32::Graphics::Direct2D::D2D1CreateFactory;
use windows::Win32::Graphics::Direct2D::ID2D1Brush;
use windows::Win32::Graphics::Direct2D::ID2D1Factory8;
use windows::Win32::Graphics::Direct2D::ID2D1HwndRenderTarget;
use windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_PER_PRIMITIVE;
use windows::Win32::Graphics::Direct2D::D2D1_BRUSH_PROPERTIES;
use windows::Win32::Graphics::Direct2D::D2D1_FACTORY_TYPE_MULTI_THREADED;
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
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
use windows::Win32::UI::WindowsAndMessaging::GetWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::SetWindowPos;
use windows::Win32::UI::WindowsAndMessaging::CREATESTRUCTW;
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT;
use windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;
use windows::Win32::UI::WindowsAndMessaging::GW_HWNDPREV;
use windows::Win32::UI::WindowsAndMessaging::HWND_TOP;
use windows::Win32::UI::WindowsAndMessaging::LWA_ALPHA;
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
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGED;
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGING;
use windows::Win32::UI::WindowsAndMessaging::WS_DISABLED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT;
use windows::Win32::UI::WindowsAndMessaging::WS_POPUP;

use super::get_borders;

static RENDER_FACTORY: LazyLock<ID2D1Factory8> = unsafe {
    LazyLock::new(|| {
        match D2D1CreateFactory::<ID2D1Factory8>(D2D1_FACTORY_TYPE_MULTI_THREADED, None) {
            Ok(factory) => factory,
            Err(err) => {
                // Not sure how I can recover from this error so I'm just going to panic
                error!("Critical Error: failed to create ID2D1Factory, {}", err);
                panic!("Failed to create ID2D1Factory, {}", err);
            }
        }
    })
};

impl TypeKind for Border {
    type TypeKind = CloneType;
}

impl Eq for Border {}

impl PartialEq for Border {
    fn eq(&self, other: &Self) -> bool {
        self.tracking_window.0 as usize == other.tracking_window.0 as usize
    }
}

#[derive(Debug, Default, Clone)]
pub struct Border {
    pub border_window: HWND,
    pub tracking_window: HWND,
    pub is_window_active: bool,
    pub window_rect: RECT,
    pub border_width: i32,
    pub border_offset: i32,
    pub border_radius: f32,
    pub render_target: Option<ID2D1HwndRenderTarget>,
    pub rounded_rect: D2D1_ROUNDED_RECT,
    pub active_color: Color,
    pub inactive_color: Color,
    pub animations: Animations,
    pub last_animation_time: Option<Instant>,
    pub last_render_time: Option<Instant>,
    pub initialize_delay: u64,
    pub unminimize_delay: u64,
    pub pause: bool,
}

impl Border {
    pub fn new(tracking_window: HWND) -> Self {
        Self {
            tracking_window,
            ..Default::default()
        }
    }

    pub fn create_border_window(&mut self, window_rule: &WindowRule) -> WinResult<()> {
        let title: Vec<u16> = format!(
            "tacky-border | {} | {:?}\0",
            WindowsApi::get_window_title(self.tracking_window).unwrap_or_default(),
            self.tracking_window
        )
        .encode_utf16()
        .collect();

        self.border_window = WindowsApi::create_window_ex_w(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
            w!("border"),
            PCWSTR(title.as_ptr()),
            WS_POPUP | WS_DISABLED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            unsafe { GetModuleHandleW(None)? },
            Some(ptr::addr_of!(*self) as _),
        )?;

        self.load_from_config(window_rule).log_if_err();

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

            DwmEnableBlurBehindWindow(self.border_window, &bh)
                .context("could not make window transparent")?;

            WindowsApi::set_layered_window_attributes(
                self.border_window,
                COLORREF(0x00000000),
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

            SetAnimationTimer(
                self,
                Some(|border: &Border| {
                    !border.animations.active.is_empty() || !border.animations.inactive.is_empty()
                }),
            )
            .log_if_err();

            let mut message = MSG::default();
            while WindowsApi::get_message_w(&mut message, HWND::default(), 0, 0).into() {
                let _ = WindowsApi::translate_message(&message);
                WindowsApi::dispatch_message_w(&message);
            }
            debug!("exiting border thread for {:?}!", self.tracking_window);
        }

        Ok(())
    }

    fn load_from_config(&mut self, window_rule: &WindowRule) -> AnyResult<()> {
        let config = CONFIG.read().unwrap();

        let config_width = window_rule
            .rule_match
            .border_width
            .unwrap_or(config.global_rule.border_width);
        let config_offset = window_rule
            .rule_match
            .border_offset
            .unwrap_or(config.global_rule.border_offset);
        let config_radius = window_rule
            .rule_match
            .border_radius
            .clone()
            .unwrap_or(config.global_rule.border_radius.clone());

        let config_active = window_rule
            .rule_match
            .active_color
            .clone()
            .unwrap_or(config.global_rule.active_color.clone());

        let config_inactive = window_rule
            .rule_match
            .inactive_color
            .clone()
            .unwrap_or(config.global_rule.inactive_color.clone());

        self.active_color = config_active.to_color(Some(true))?;
        self.inactive_color = config_inactive.to_color(Some(false))?;

        self.animations = window_rule
            .rule_match
            .animations
            .clone()
            .unwrap_or(config.global_rule.animations.clone().unwrap_or_default());

        let dpi = unsafe { GetDpiForWindow(self.tracking_window) } as f32;
        self.border_width = (config_width * dpi / 96.0) as i32;
        self.border_radius = config_radius.parse(self.border_width, dpi, self.tracking_window);
        self.border_offset = config_offset;

        let available_windows = WindowsApi::collect_window_handles().unwrap_or_default();

        self.initialize_delay = match available_windows.contains(&(self.tracking_window.0 as isize))
        {
            true => 0,
            false => window_rule
                .rule_match
                .initialize_delay
                .unwrap_or(config.global_rule.initialize_delay.unwrap_or(250)),
        };

        self.unminimize_delay = window_rule
            .rule_match
            .unminimize_delay
            .unwrap_or(config.global_rule.unminimize_delay.unwrap_or(200));

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
            hwnd: self.border_window,
            pixelSize: Default::default(),
            presentOptions: D2D1_PRESENT_OPTIONS_RETAIN_CONTENTS | D2D1_PRESENT_OPTIONS_IMMEDIATELY,
        };

        let brush_properties = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,
            transform: Matrix3x2::identity(),
        };

        self.rounded_rect = D2D1_ROUNDED_RECT {
            rect: Default::default(),
            radiusX: self.border_radius,
            radiusY: self.border_radius,
        };

        // Initialize the actual border color assuming it is in focus
        unsafe {
            let render_target = RENDER_FACTORY.CreateHwndRenderTarget(
                &render_target_properties,
                &hwnd_render_target_properties,
            )?;

            render_target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);

            self.active_color
                .to_d2d1_brush(&render_target, &self.window_rect, &brush_properties)
                .log_if_err();
            self.inactive_color
                .to_d2d1_brush(&render_target, &self.window_rect, &brush_properties)
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

        self.window_rect.top -= self.border_width;
        self.window_rect.left -= self.border_width;
        self.window_rect.right += self.border_width;
        self.window_rect.bottom += self.border_width;

        Ok(())
    }

    fn update_position(&mut self, other_flags: Option<SET_WINDOW_POS_FLAGS>) -> AnyResult<()> {
        unsafe {
            // Place the window border above the tracking window
            let hwnd_above_tracking = GetWindow(self.tracking_window, GW_HWNDPREV);

            let mut swp_flags = SWP_NOSENDCHANGING
                | SWP_NOACTIVATE
                | SWP_NOREDRAW
                | other_flags.unwrap_or_default();

            if hwnd_above_tracking == Ok(self.border_window) {
                swp_flags |= SWP_NOZORDER;
            }

            if let Err(e) = SetWindowPos(
                self.border_window,
                hwnd_above_tracking.unwrap_or(HWND_TOP),
                self.window_rect.left,
                self.window_rect.top,
                WindowsApi::get_rect_width(self.window_rect),
                WindowsApi::get_rect_height(self.window_rect),
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
        match self.current_animations().contains_key(&AnimationType::Fade) {
            false => self.update_brush_opacities(),
            true if check_delay == Some(0) => {
                self.update_brush_opacities();
                self.refresh_fade_progress();
            }
            true => self.animations.flags.should_fade = true,
        }

        Ok(())
    }

    fn refresh_fade_progress(&mut self) {
        self.animations.progress.fade = match self.is_window_active {
            true => 1.0,
            false => 0.0,
        };
    }

    fn update_brush_opacities(&mut self) {
        let (top_color, bottom_color) = match self.is_window_active {
            true => (&mut self.active_color, &mut self.inactive_color),
            false => (&mut self.inactive_color, &mut self.active_color),
        };
        top_color.set_opacity(1.0);
        bottom_color.set_opacity(0.0);
    }

    fn current_animations(&self) -> &FxHashMap<AnimationType, AnimationParameters> {
        match self.is_window_active {
            true => &self.animations.active,
            false => &self.animations.inactive,
        }
    }

    fn render(&mut self) -> AnyResult<()> {
        self.last_render_time = Some(std::time::Instant::now());

        let Some(ref render_target) = self.render_target else {
            return Err(anyhow!("render_target has not been set yet"));
        };

        let rect_width = WindowsApi::get_rect_width(self.window_rect) as f32;
        let rect_height = WindowsApi::get_rect_height(self.window_rect) as f32;

        let pixel_size = D2D_SIZE_U {
            width: rect_width as u32,
            height: rect_height as u32,
        };

        let border_width = self.border_width as f32;
        let border_offset = self.border_offset as f32;

        self.rounded_rect.rect = D2D_RECT_F {
            left: border_width / 2.0 - border_offset,
            top: border_width / 2.0 - border_offset,
            right: rect_width - border_width / 2.0 + border_offset,
            bottom: rect_height - border_width / 2.0 + border_offset,
        };

        unsafe {
            render_target
                .Resize(&pixel_size)
                .context("could not resize render_target")?;

            let (bottom_color, top_color) = match self.is_window_active {
                true => (&self.inactive_color, &self.active_color),
                false => (&self.active_color, &self.inactive_color),
            };

            render_target.BeginDraw();
            render_target.Clear(None);

            if bottom_color.get_opacity() > Some(0.0) {
                if let Color::Gradient(gradient) = bottom_color {
                    gradient.update_start_end_points(&self.window_rect);
                }

                match bottom_color.get_brush() {
                    Some(id2d1_brush) => self.draw_rectangle(render_target, id2d1_brush),
                    None => debug!("ID2D1Brush for bottom_color has not been created yet"),
                }
            }

            if top_color.get_opacity() > Some(0.0) {
                if let Color::Gradient(gradient) = top_color {
                    gradient.update_start_end_points(&self.window_rect);
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
        unsafe {
            match self.border_radius {
                0.0 => render_target.DrawRectangle(
                    &self.rounded_rect.rect,
                    brush,
                    self.border_width as f32,
                    None,
                ),
                _ => render_target.DrawRoundedRectangle(
                    &self.rounded_rect,
                    brush,
                    self.border_width as f32,
                    None,
                ),
            }
        }
    }

    fn exit_border_thread(&mut self) {
        self.pause = true;
        KillAnimationTimer(self).log_if_err();
        let mut borders_hashmap = get_borders();
        borders_hashmap.remove(&(self.tracking_window.0 as isize));

        drop(borders_hashmap);
        WindowsApi::post_quit_message(0);
    }

    fn callback(&mut self, window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match message {
            // EVENT_OBJECT_LOCATIONCHANGE
            WM_APP_LOCATIONCHANGE => {
                if self.pause {
                    return LRESULT(0);
                }
                if !WindowsApi::has_native_border(self.tracking_window) {
                    self.update_position(Some(SWP_HIDEWINDOW)).log_if_err();
                    return LRESULT(0);
                }

                let old_rect = self.window_rect;
                self.update_window_rect().log_if_err();

                let update_pos_flags = match WindowsApi::is_window_visible(self.border_window) {
                    true => None,
                    false => Some(SWP_SHOWWINDOW),
                };

                self.update_position(update_pos_flags).log_if_err();

                // TODO When a window is minimized, all four points of the rect go way below 0. For
                // some reason, after unminimizing/restoring, render() will sometimes render at
                // this minimized size. self.window_rect = old_rect is hopefully only a temporary solution.
                match WindowsApi::is_rect_visible(&self.window_rect) {
                    true => {
                        if !WindowsApi::are_rects_same_size(&self.window_rect, &old_rect) {
                            self.render().log_if_err();
                        }
                    }
                    false => self.window_rect = old_rect,
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
                self.is_window_active = WindowsApi::is_window_active(self.tracking_window);

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

                if !WindowsApi::is_rect_visible(&self.window_rect) {
                    self.window_rect = old_rect;
                    return LRESULT(0);
                }

                if WindowsApi::has_native_border(self.tracking_window) {
                    self.update_position(Some(SWP_SHOWWINDOW)).log_if_err();
                    self.render().log_if_err();
                }

                SetAnimationTimer(
                    self,
                    Some(|border: &Border| {
                        !border.animations.active.is_empty()
                            || !border.animations.inactive.is_empty()
                    }),
                )
                .log_if_err();

                self.pause = false;
            }
            // EVENT_OBJECT_HIDE / EVENT_OBJECT_CLOAKED
            WM_APP_HIDECLOAKED => {
                self.update_position(Some(SWP_HIDEWINDOW)).log_if_err();
                KillAnimationTimer(self).log_if_err();
                self.pause = true;
            }
            // EVENT_OBJECT_MINIMIZESTART
            WM_APP_MINIMIZESTART => {
                self.update_position(Some(SWP_HIDEWINDOW)).log_if_err();

                self.active_color.set_opacity(0.0);
                self.inactive_color.set_opacity(0.0);

                KillAnimationTimer(self).log_if_err();
                self.pause = true;
            }
            // EVENT_SYSTEM_MINIMIZEEND
            // When a window is about to be unminimized, hide the border and let the thread sleep
            // to wait for the window animation to finish, then show the border.
            WM_APP_MINIMIZEEND => {
                thread::sleep(time::Duration::from_millis(self.unminimize_delay));

                self.last_animation_time = Some(time::Instant::now());

                if WindowsApi::has_native_border(self.tracking_window) {
                    self.update_color(Some(self.unminimize_delay)).log_if_err();
                    self.update_window_rect().log_if_err();
                    self.update_position(Some(SWP_SHOWWINDOW)).log_if_err();
                    self.render().log_if_err();
                }

                SetAnimationTimer(
                    self,
                    Some(|border: &Border| {
                        !border.animations.active.is_empty()
                            || !border.animations.inactive.is_empty()
                    }),
                )
                .log_if_err();

                self.pause = false;
            }
            WM_APP_TIMER => {
                if self.pause {
                    return LRESULT(0);
                }

                let anim_elapsed = self
                    .last_animation_time
                    .unwrap_or(time::Instant::now())
                    .elapsed();
                let render_elapsed = self
                    .last_render_time
                    .unwrap_or(time::Instant::now())
                    .elapsed();

                self.last_animation_time = Some(time::Instant::now());

                let mut animations_updated = false;

                let current_animations = self.current_animations();

                if current_animations.clone().is_empty() {
                    self.active_color.set_transform(&Matrix3x2::identity());
                    self.inactive_color.set_transform(&Matrix3x2::identity());
                    animations_updated = false;
                } else {
                    for (anim_type, anim_value) in current_animations.clone().iter() {
                        match anim_type {
                            AnimationType::Spiral | AnimationType::ReverseSpiral => {
                                anim_value.play(anim_type, self, &anim_elapsed);
                                animations_updated = true;
                            }
                            AnimationType::Fade => {
                                if self.animations.flags.should_fade {
                                    anim_value.play(anim_type, self, &anim_elapsed);
                                    animations_updated = true;
                                }
                            }
                        }
                    }
                }

                // println!("time since last anim: {}", render_elapsed.as_secs_f32());

                let interval = 1.0 / self.animations.fps as f32;
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
