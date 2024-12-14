use crate::animations::animation::AnimationType;
use crate::animations::timer::AnimationTimer;
use crate::animations::Animations;
use crate::animations::ANIM_FADE;
use crate::utils::LogIfErr;
use crate::windows_api::ErrorMsg;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_FOREGROUND;
use crate::windows_api::WM_APP_HIDECLOAKED;
use crate::windows_api::WM_APP_LOCATIONCHANGE;
use crate::windows_api::WM_APP_MINIMIZEEND;
use crate::windows_api::WM_APP_MINIMIZESTART;
use crate::windows_api::WM_APP_REORDER;
use crate::windows_api::WM_APP_SHOWUNCLOAKED;
use crate::windows_api::WM_APP_TIMER;
use crate::BORDERS;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result as AnyResult;
use std::ptr;
use std::sync::LazyLock;
use std::thread;
use std::time;
use win_color::Color;
use win_color::ColorImpl;
use win_color::GradientImpl;
use windows::core::w;
use windows::core::Result as WinResult;
use windows::core::PCWSTR;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Foundation::D2DERR_RECREATE_TARGET;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Foundation::HINSTANCE;
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
use windows::Win32::UI::WindowsAndMessaging::CreateWindowExW;
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
use windows::Win32::UI::WindowsAndMessaging::GetWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
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
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGED;
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGING;
use windows::Win32::UI::WindowsAndMessaging::WS_DISABLED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT;
use windows::Win32::UI::WindowsAndMessaging::WS_POPUP;

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

#[derive(Debug, Default)]
pub struct WindowBorder {
    pub border_window: HWND,
    pub tracking_window: HWND,
    pub window_rect: RECT,
    pub border_width: i32,
    pub border_offset: i32,
    pub border_radius: f32,
    pub brush_properties: D2D1_BRUSH_PROPERTIES,
    pub render_target: Option<ID2D1HwndRenderTarget>,
    pub rounded_rect: D2D1_ROUNDED_RECT,
    pub animations: Animations,
    pub active_color: Color,
    pub inactive_color: Color,
    pub initialize_delay: u64,
    pub unminimize_delay: u64,
    pub pause: bool,
    pub last_animation_time: Option<std::time::Instant>,
    pub last_render_time: Option<std::time::Instant>,
    pub animation_timer: Option<AnimationTimer>,
    pub event_anim: i32,
    pub is_window_active: bool,
}

impl WindowBorder {
    pub fn create_border_window(&mut self, hinstance: HINSTANCE) -> WinResult<()> {
        let self_title = format!(
            "{} | {} | {:?}",
            "tacky-border",
            WindowsApi::get_window_title(self.tracking_window),
            self.tracking_window
        );
        let mut string: Vec<u16> = self_title.encode_utf16().collect();
        string.push(0);
        unsafe {
            self.border_window = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                w!("border"),
                PCWSTR::from_raw(string.as_ptr()),
                WS_POPUP | WS_DISABLED,
                0,
                0,
                0,
                0,
                None,
                None,
                hinstance,
                Some(ptr::addr_of!(*self) as *const _),
            )?;
        }

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

            let _ = DwmEnableBlurBehindWindow(self.border_window, &bh);
            let _ = WindowsApi::set_layered_window_attributes::<fn()>(
                self.border_window,
                COLORREF(0x00000000),
                0,
                LWA_COLORKEY,
                None,
            );

            let _ = WindowsApi::set_layered_window_attributes::<fn()>(
                self.border_window,
                COLORREF(0x00000000),
                255,
                LWA_ALPHA,
                None,
            );

            self.create_render_targets()
                .context("could not create render target in init()")?;

            self.is_window_active = WindowsApi::is_window_active(self.tracking_window);

            self.animations.current = match self.is_window_active {
                true => self.animations.active.clone(),
                false => self.animations.inactive.clone(),
            };

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

            self.set_anim_timer();

            let mut message = MSG::default();
            while WindowsApi::get_message_w(&mut message, HWND::default(), 0, 0).into() {
                let _ = WindowsApi::translate_message(&message);
                WindowsApi::dispatch_message_w(&message);
            }
            debug!("exiting border thread for {:?}!", self.tracking_window);
        }

        Ok(())
    }

    fn create_render_targets(&mut self) -> AnyResult<()> {
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
        let _ = WindowsApi::dwm_get_window_attribute::<RECT, _>(
            self.tracking_window,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            ptr::addr_of_mut!(self.window_rect) as _,
            Some(ErrorMsg::Fn(|| {
                self.destroy_anim_timer();
                self.exit_border_thread();
            })),
        );

        self.window_rect.top -= self.border_width;
        self.window_rect.left -= self.border_width;
        self.window_rect.right += self.border_width;
        self.window_rect.bottom += self.border_width;

        Ok(())
    }

    fn update_position(&mut self, c_flags: Option<SET_WINDOW_POS_FLAGS>) -> AnyResult<()> {
        unsafe {
            // Place the window border above the tracking window
            let hwnd_above_tracking = GetWindow(self.tracking_window, GW_HWNDPREV);
            let mut u_flags =
                SWP_NOSENDCHANGING | SWP_NOACTIVATE | SWP_NOREDRAW | c_flags.unwrap_or_default();

            if hwnd_above_tracking == Ok(self.border_window) {
                u_flags |= SWP_NOZORDER;
            }

            if let Err(e) = SetWindowPos(
                self.border_window,
                hwnd_above_tracking.unwrap_or(HWND_TOP),
                self.window_rect.left,
                self.window_rect.top,
                WindowsApi::get_rect_width(self.window_rect),
                WindowsApi::get_rect_height(self.window_rect),
                u_flags,
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
        match self.animations.current.contains_key(&AnimationType::Fade) && check_delay != Some(0) {
            true => {
                self.event_anim = ANIM_FADE;
            }
            false => {
                self.animations.fade_progress = match self.is_window_active {
                    true => 1.0,
                    false => 0.0,
                };
                let (top_color, bottom_color) = match self.is_window_active {
                    true => (&mut self.active_color, &mut self.inactive_color),
                    false => (&mut self.inactive_color, &mut self.active_color),
                };
                top_color.set_opacity(1.0);
                bottom_color.set_opacity(0.0);
            }
        }

        Ok(())
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

        let width = self.border_width as f32;
        let offset = self.border_offset as f32;

        self.rounded_rect.rect = D2D_RECT_F {
            left: width / 2.0 - offset,
            top: width / 2.0 - offset,
            right: rect_width - width / 2.0 + offset,
            bottom: rect_height - width / 2.0 + offset,
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

                    match self.create_render_targets() {
                        Ok(_) => info!("Successfully recreated render_target; resuming thread"),
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

    fn set_anim_timer(&mut self) {
        if (!self.animations.active.is_empty() || !self.animations.inactive.is_empty())
            && self.animation_timer.is_none()
        {
            let timer_duration = (1000.0 / self.animations.fps as f32) as u64;
            self.animation_timer = Some(AnimationTimer::start(self.border_window, timer_duration));
        }
    }

    fn destroy_anim_timer(&mut self) {
        if let Some(anim_timer) = self.animation_timer.as_mut() {
            anim_timer.stop();
            self.animation_timer = None;
        }
    }

    fn exit_border_thread(&mut self) {
        self.pause = true;
        self.destroy_anim_timer();
        BORDERS
            .lock()
            .unwrap()
            .remove(&(self.tracking_window.0 as isize));
        unsafe { PostQuitMessage(0) };
    }

    pub unsafe extern "system" fn s_wnd_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let mut border_pointer: *mut WindowBorder = GetWindowLongPtrW(window, GWLP_USERDATA) as _;

        if border_pointer.is_null() && message == WM_CREATE {
            let create_struct: *mut CREATESTRUCTW = lparam.0 as *mut _;
            border_pointer = (*create_struct).lpCreateParams as *mut _;
            SetWindowLongPtrW(window, GWLP_USERDATA, border_pointer as _);
        }
        match !border_pointer.is_null() {
            true => Self::wnd_proc(&mut *border_pointer, window, message, wparam, lparam),
            false => DefWindowProcW(window, message, wparam, lparam),
        }
    }

    pub unsafe fn wnd_proc(
        &mut self,
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
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
            WM_APP_FOREGROUND => {
                self.is_window_active = WindowsApi::is_window_active(self.tracking_window);

                self.animations.current = match self.is_window_active {
                    true => self.animations.active.clone(),
                    false => self.animations.inactive.clone(),
                };

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

                self.set_anim_timer();

                self.pause = false;
            }
            // EVENT_OBJECT_HIDE / EVENT_OBJECT_CLOAKED
            WM_APP_HIDECLOAKED => {
                let _ = self.update_position(Some(SWP_HIDEWINDOW));

                self.destroy_anim_timer();

                self.pause = true;
            }
            // EVENT_OBJECT_MINIMIZESTART
            WM_APP_MINIMIZESTART => {
                self.update_position(Some(SWP_HIDEWINDOW)).log_if_err();

                // TODO this is scuffed to work with fade animations
                self.active_color.set_opacity(0.0);
                self.inactive_color.set_opacity(0.0);

                self.destroy_anim_timer();

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

                self.set_anim_timer();

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

                if self.animations.current.is_empty() {
                    self.brush_properties.transform = Matrix3x2::identity();
                    animations_updated = false;
                } else {
                    for (anim_type, anim_value) in self.animations.current.clone().iter() {
                        match anim_type {
                            AnimationType::Spiral | AnimationType::ReverseSpiral => {
                                anim_value.play(anim_type, self, &anim_elapsed);
                                animations_updated = true;
                            }
                            AnimationType::Fade => {
                                if self.event_anim == ANIM_FADE {
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
                let _ = ValidateRect(window, None);
            }
            WM_NCDESTROY => {
                SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                self.exit_border_thread();
            }
            // Ignore these window position messages
            WM_WINDOWPOSCHANGING | WM_WINDOWPOSCHANGED => {}
            _ => {
                return DefWindowProcW(window, message, wparam, lparam);
            }
        }
        LRESULT(0)
    }
}
