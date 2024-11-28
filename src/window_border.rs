use crate::animations::AnimationType;
use crate::animations::Animations;
use crate::animations::HashMapAnimationExt;
use crate::animations::ANIM_FADE;
use crate::colors::color::Color;
use crate::timer::KillCustomTimer;
use crate::timer::SetCustomTimer;
use crate::windows_api::ErrorMsg;
use crate::windows_api::WindowsApi;
use crate::windows_api::WM_APP_FOCUS;
use crate::windows_api::WM_APP_HIDECLOAKED;
use crate::windows_api::WM_APP_LOCATIONCHANGE;
use crate::windows_api::WM_APP_MINIMIZEEND;
use crate::windows_api::WM_APP_MINIMIZESTART;
use crate::windows_api::WM_APP_REORDER;
use crate::windows_api::WM_APP_SHOWUNCLOAKED;
use crate::windows_api::WM_APP_TIMER;

use std::ptr;
use std::sync::LazyLock;
use std::sync::OnceLock;
use std::thread;
use std::time;

use windows::core::w;
use windows::core::Result;

use windows::core::PCWSTR;
use windows::Foundation::Numerics::Matrix3x2;

use windows::Win32::Foundation::COLORREF;
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
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
use windows::Win32::UI::WindowsAndMessaging::GetWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::SetWindowPos;
use windows::Win32::UI::WindowsAndMessaging::ShowWindow;
use windows::Win32::UI::WindowsAndMessaging::TranslateMessage;
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
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNA;
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

pub static RENDER_FACTORY: LazyLock<ID2D1Factory8> = unsafe {
    LazyLock::new(|| {
        D2D1CreateFactory::<ID2D1Factory8>(D2D1_FACTORY_TYPE_MULTI_THREADED, None)
            .expect("creating RENDER_FACTORY failed")
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
    pub render_target: OnceLock<ID2D1HwndRenderTarget>,
    pub rounded_rect: D2D1_ROUNDED_RECT,
    pub animations: Animations,
    pub active_color: Color,
    pub inactive_color: Color,
    pub current_color: Color,
    pub unminimize_delay: u64,
    pub pause: bool,
    pub last_animation_time: Option<std::time::Instant>,
    pub last_render_time: Option<std::time::Instant>,
    pub spiral_anim_angle: f32,
    pub event_anim: i32,
    pub is_window_active: bool,
    pub timer_id: Option<usize>,
}

impl WindowBorder {
    pub fn create_border_window(&mut self, hinstance: HINSTANCE) -> Result<()> {
        unsafe {
            let self_title = format!(
                "{}{}",
                "tacky-",
                WindowsApi::get_window_title(self.tracking_window)
            );
            let mut string: Vec<u16> = self_title.encode_utf16().collect();
            string.push(0);
            self.border_window = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                w!("tacky-border"),
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

    pub fn init(&mut self, initialize_delay: u64) -> Result<()> {
        thread::sleep(time::Duration::from_millis(initialize_delay));

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

            let _ = self.create_render_targets();

            self.is_window_active = WindowsApi::is_window_active(self.tracking_window);

            self.animations.current = match self.is_window_active {
                true => self.animations.active.clone(),
                false => self.animations.inactive.clone(),
            };

            let _ = self.update_color(Some(initialize_delay));

            let _ = self.update_window_rect();

            if WindowsApi::has_native_border(self.tracking_window) {
                let _ = self.update_position(Some(SWP_SHOWWINDOW));
                let _ = self.render();

                thread::sleep(time::Duration::from_millis(5));
                let _ = self.update_position(Some(SWP_SHOWWINDOW));
                let _ = self.render();
            }

            self.set_anim_timer();

            let mut message = MSG::default();
            while GetMessageW(&mut message, HWND::default(), 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            debug!("exiting border thread for {:?}!", self.tracking_window);
        }

        Ok(())
    }

    pub fn create_render_targets(&mut self) -> Result<()> {
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
            presentOptions: D2D1_PRESENT_OPTIONS_IMMEDIATELY,
        };
        self.brush_properties = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,
            transform: Matrix3x2::identity(),
        };

        self.rounded_rect = D2D1_ROUNDED_RECT {
            rect: Default::default(),
            radiusX: self.border_radius,
            radiusY: self.border_radius,
        };

        if WindowsApi::is_window_active(self.tracking_window) {
            self.current_color = self.active_color.clone();
        } else {
            self.current_color = self.inactive_color.clone();
        }

        // Initialize the actual border color assuming it is in focus
        unsafe {
            let factory = &*RENDER_FACTORY;
            let _ = self.render_target.set(
                factory
                    .CreateHwndRenderTarget(
                        &render_target_properties,
                        &hwnd_render_target_properties,
                    )
                    .expect("creating self.render_target failed"),
            );
            let render_target = self.render_target.get().unwrap();
            render_target.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        }

        Ok(())
    }

    pub fn update_window_rect(&mut self) -> Result<()> {
        let _ = WindowsApi::dwm_get_window_attribute::<RECT, _>(
            self.tracking_window,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            ptr::addr_of_mut!(self.window_rect) as _,
            Some(ErrorMsg::Fn(|| {
                unsafe {
                    // Temporarily borrow self mutably here, outside of the closure
                    self.destroy_anim_timer();
                    PostQuitMessage(0);
                }
            })),
        );

        self.window_rect.top -= self.border_width;
        self.window_rect.left -= self.border_width;
        self.window_rect.right += self.border_width;
        self.window_rect.bottom += self.border_width;

        Ok(())
    }

    pub fn update_position(&mut self, c_flags: Option<SET_WINDOW_POS_FLAGS>) -> Result<()> {
        unsafe {
            // Place the window border above the tracking window
            let hwnd_above_tracking = GetWindow(self.tracking_window, GW_HWNDPREV);
            let mut u_flags =
                SWP_NOSENDCHANGING | SWP_NOACTIVATE | SWP_NOREDRAW | c_flags.unwrap_or_default();

            if hwnd_above_tracking == Ok(self.border_window) {
                u_flags |= SWP_NOZORDER;
            }

            let result = SetWindowPos(
                self.border_window,
                hwnd_above_tracking.unwrap_or(HWND_TOP),
                self.window_rect.left,
                self.window_rect.top,
                WindowsApi::get_rect_width(self.window_rect),
                WindowsApi::get_rect_height(self.window_rect),
                u_flags,
            );
            if result.is_err() {
                warn!("Could not set window position! This is normal for elevated/admin windows.");
                self.destroy_anim_timer();
                PostQuitMessage(0);
            }
        }
        Ok(())
    }

    pub fn update_color(&mut self, check_delay: Option<u64>) -> Result<()> {
        match self.animations.current.has(&AnimationType::Fade) && check_delay != Some(0) {
            true => {
                self.event_anim = ANIM_FADE;
            }
            false => {
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

    pub fn render(&mut self) -> Result<()> {
        self.last_render_time = Some(std::time::Instant::now());

        let Some(render_target) = self.render_target.get() else {
            return Ok(());
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
            let _ = render_target.Resize(&pixel_size);

            let active_opacity = self.active_color.get_opacity();
            let inactive_opacity = self.inactive_color.get_opacity();

            let (bottom_opacity, top_opacity) = match self.is_window_active {
                true => (inactive_opacity, active_opacity),
                false => (active_opacity, inactive_opacity),
            };

            let (bottom_color, top_color) = match self.is_window_active {
                true => (&self.inactive_color, &self.active_color),
                false => (&self.active_color, &self.inactive_color),
            };

            render_target.BeginDraw();
            render_target.Clear(None);

            if bottom_opacity > 0.0 {
                let Some(brush) =
                    bottom_color.to_brush(render_target, &self.window_rect, &self.brush_properties)
                else {
                    return Ok(());
                };
                self.draw_rectangle(render_target, &brush);
            }
            if top_opacity > 0.0 {
                let Some(brush) =
                    top_color.to_brush(render_target, &self.window_rect, &self.brush_properties)
                else {
                    return Ok(());
                };
                self.draw_rectangle(render_target, &brush);
            }

            let _ = render_target.EndDraw(None, None);
        }

        Ok(())
    }

    pub fn draw_rectangle(&self, render_target: &ID2D1HwndRenderTarget, brush: &ID2D1Brush) {
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

    pub fn set_anim_timer(&mut self) {
        if !self.animations.active.is_empty()
            || !self.animations.inactive.is_empty() && self.timer_id.is_none()
        {
            let timer_duration = (1000 / self.animations.fps) as u32;
            unsafe {
                self.timer_id = Some(SetCustomTimer(self.border_window, 1, timer_duration));
            }
        }
    }

    pub fn destroy_anim_timer(&mut self) {
        KillCustomTimer(self.border_window, 1);
        self.timer_id = None;
    }

    // When CreateWindowExW is called, we can optionally pass a value to its LPARAM field which will
    // get sent to the window process on creation. In our code, we've passed a pointer to the
    // WindowBorder structure during the window creation process, and here we are getting that pointer
    // and attaching it to the window using SetWindowLongPtrW.
    pub unsafe extern "system" fn s_wnd_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let mut border_pointer: *mut WindowBorder = GetWindowLongPtrW(window, GWLP_USERDATA) as _;

        if border_pointer.is_null() && message == WM_CREATE {
            //println!("ref is null, assigning new ref");
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
                    let _ = self.update_position(Some(SWP_HIDEWINDOW));
                    return LRESULT(0);
                }

                if !WindowsApi::is_window_visible(self.border_window) {
                    let _ = ShowWindow(self.border_window, SW_SHOWNA);
                }

                let old_rect = self.window_rect;
                let _ = self.update_window_rect();
                let _ = self.update_position(None);

                // TODO When a window is minimized, all four points of the rect go way below 0. For
                // some reason, after unminimizing/restoring, render() will sometimes render at
                // this minimized size. self.window_rect = old_rect is hopefully only a temporary solution.
                if !WindowsApi::is_rect_visible(&self.window_rect) {
                    self.window_rect = old_rect;
                } else if !WindowsApi::are_rects_same_size(&self.window_rect, &old_rect) {
                    // Only re-render the border when its size changes
                    let _ = self.render();
                }
            }
            // EVENT_OBJECT_REORDER
            WM_APP_REORDER => {
                if self.pause {
                    return LRESULT(0);
                }

                // For apps like firefox, when you hover over a tab, a popup window spawns that
                // changes the z-order and causes the border to sit under the tracking window. To
                // remedy that, we just re-update the position/z-order when windows are reordered.
                let _ = self.update_position(None);
            }
            WM_APP_FOCUS => {
                self.is_window_active = WindowsApi::is_window_active(self.tracking_window);

                self.animations.current = match self.is_window_active {
                    true => self.animations.active.clone(),
                    false => self.animations.inactive.clone(),
                };

                let _ = self.update_color(None);
                let _ = self.update_position(None);
                let _ = self.render();
            }
            // EVENT_OBJECT_SHOW / EVENT_OBJECT_UNCLOAKED
            WM_APP_SHOWUNCLOAKED => {
                // With GlazeWM, if I switch to another workspace while a window is minimized and
                // switch back, then we will receive this message even though the window is not yet
                // visible. And, the window rect will be all weird. So, we apply the following fix.
                let old_rect = self.window_rect;
                let _ = self.update_window_rect();
                if !WindowsApi::is_rect_visible(&self.window_rect) {
                    self.window_rect = old_rect;
                    return LRESULT(0);
                }

                if WindowsApi::has_native_border(self.tracking_window) {
                    let _ = self.update_position(Some(SWP_SHOWWINDOW));
                    let _ = self.render();
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
                let _ = self.update_position(Some(SWP_HIDEWINDOW));

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
                    let _ = self.update_color(Some(self.unminimize_delay));
                    let _ = self.update_window_rect();
                    let _ = self.update_position(Some(SWP_SHOWWINDOW));
                    let _ = self.render();
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

                let mut animations_updated = if self.animations.current.is_empty() {
                    self.brush_properties.transform = Matrix3x2::identity();
                    false
                } else {
                    self.animations.current.clone().to_iter().any(|animation| {
                        match animation.animation_type {
                            AnimationType::Spiral | AnimationType::ReverseSpiral => {
                                let anim_speed = animation.speed;
                                animation.play(self, &anim_elapsed, anim_speed * 2.0);
                                true
                            }
                            _ => false,
                        }
                    })
                };

                if self.event_anim == ANIM_FADE {
                    let anim = self
                        .animations
                        .current
                        .find(&AnimationType::Fade)
                        .unwrap()
                        .clone();
                    anim.play(self, &anim_elapsed, anim.speed / 20.0);
                    animations_updated = true;
                }

                // println!("time since last anim: {}", render_elapsed.as_secs_f32());

                let interval = 1.0 / self.animations.fps as f32;
                let diff = render_elapsed.as_secs_f32() - interval;
                if animations_updated && (diff.abs() <= 0.001 || diff >= 0.0) {
                    let _ = self.render();
                }
            }
            WM_PAINT => {
                let _ = ValidateRect(window, None);
            }
            WM_NCDESTROY => {
                self.destroy_anim_timer();

                SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                PostQuitMessage(0);
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
