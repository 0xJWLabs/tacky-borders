use crate::colors::*;
use crate::winapi::*;
use crate::*;
use log:: *;
use std::ptr;
use std::sync::LazyLock;
use std::sync::OnceLock;
use std::thread;
use std::time;
use windows::{
    Foundation::Numerics::*, Win32::Graphics::Direct2D::Common::*, Win32::Graphics::Direct2D::*,
    Win32::Graphics::Dwm::*, Win32::Graphics::Dxgi::Common::*, Win32::Graphics::Gdi::*,
};

pub static RENDER_FACTORY: LazyLock<ID2D1Factory> = unsafe {
    LazyLock::new(|| {
        D2D1CreateFactory::<ID2D1Factory>(D2D1_FACTORY_TYPE_MULTI_THREADED, None)
            .expect("creating RENDER_FACTORY failed")
    })
};

#[derive(Debug, Default)]
pub struct WindowBorder {
    pub border_window: HWND,
    pub tracking_window: HWND,
    pub window_rect: RECT,
    pub border_size: i32,
    pub border_offset: i32,
    pub border_radius: f32,
    pub brush_properties: D2D1_BRUSH_PROPERTIES,
    pub render_target: OnceLock<ID2D1HwndRenderTarget>,
    pub rounded_rect: D2D1_ROUNDED_RECT,
    pub active_color: Color,
    pub inactive_color: Color,
    pub current_color: Color,
    pub unminimize_delay: u64,
    pub pause: bool,
    pub active_gradient_angle: f32,
    pub inactive_gradient_angle: f32,
    pub last_render_time_active: Option<std::time::Instant>,
    pub last_render_time_inactive: Option<std::time::Instant>,
    pub use_active_animation: bool,
    pub use_inactive_animation: bool,
    pub timer_id: Option<i8>,
}

impl WindowBorder {
    pub fn create_border_window(&mut self, hinstance: HINSTANCE) -> Result<()> {
        unsafe {
            self.border_window = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                w!("tacky-border"),
                w!("tacky-border"),
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

    pub fn init(&mut self, init_delay: u64) -> Result<()> {
        thread::sleep(time::Duration::from_millis(init_delay));

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

            if SetLayeredWindowAttributes(self.border_window, COLORREF(0x00000000), 0, LWA_COLORKEY)
                .is_err()
            {
                error!("Error setting layered window attributes!");
            }
            if SetLayeredWindowAttributes(self.border_window, COLORREF(0x00000000), 255, LWA_ALPHA)
                .is_err()
            {
                error!("Error setting layered window attributes!");
            }

            let _ = self.create_render_targets();
            if WinApi::has_native_border(self.tracking_window) {
                let _ = self.update_position(Some(SWP_SHOWWINDOW));
                let _ = self.render();

                // Sometimes, it doesn't show the window at first, so we wait 5ms and update it.
                // This is very hacky and needs to be looked into. It may be related to the issue
                // detailed in update_window_rect. TODO
                while !WinApi::is_window_visible(self.tracking_window) {
                    thread::sleep(std::time::Duration::from_millis(5))
                }
                let _ = self.update_position(Some(SWP_SHOWWINDOW));
                let _ = self.render();
            }

            let mut message = MSG::default();

            while GetMessageW(&mut message, HWND::default(), 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            debug!("{}", format!("exiting border thread for {:?}!", self.tracking_window));
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
        self.active_gradient_angle = 0.0;
        self.inactive_gradient_angle = 0.0;
        self.last_render_time_active = Some(std::time::Instant::now());
        self.last_render_time_inactive = Some(std::time::Instant::now());
        self.brush_properties = D2D1_BRUSH_PROPERTIES {
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

        let _ = self.update_color();
        let _ = self.update_window_rect();
        let _ = self.update_position(None);
        let _ = self.create_animation_thread();

        Ok(())
    }

    pub fn create_animation_thread(&self) -> Result<()> {
        if self.use_active_animation || self.use_inactive_animation {
            let window_sent: SendHWND = SendHWND(self.border_window);
            std::thread::spawn(move || {
                let window = window_sent;
                if WinApi::is_window_visible(window.0) {
                    unsafe {
                        // Post initial WM_PAINT to start the rendering process
                        let _ = PostMessageW(window.0, WM_PAINT, WPARAM(0), LPARAM(0));
                    }
                }
            });
        }

        Ok(())
    }

    pub fn update_window_rect(&mut self) -> Result<()> {
        let _ = WinApi::dwm_get_window_attribute(self.tracking_window, 
            DWMWA_EXTENDED_FRAME_BOUNDS, 
            &mut self.window_rect, 
            Some(ErrorMsg::Fn(|| {
                error!("Error getting frame rect!");
                unsafe {
                    let _ = ShowWindow(self.border_window, SW_HIDE);
                }
            }))
        );

        self.window_rect.top -= self.border_size;
        self.window_rect.left -= self.border_size;
        self.window_rect.right += self.border_size;
        self.window_rect.bottom += self.border_size;

        Ok(())
    }

    pub fn update_position(&mut self, c_flags: Option<SET_WINDOW_POS_FLAGS>) -> Result<()> {
        unsafe {
            // Place the window border above the tracking window
            let hwnd_above_tracking = GetWindow(self.tracking_window, GW_HWNDPREV);
            let mut u_flags = SWP_NOSENDCHANGING | SWP_NOACTIVATE | SWP_NOREDRAW | c_flags.unwrap_or_default();

            if hwnd_above_tracking == Ok(self.border_window) {
                u_flags |= SWP_NOZORDER;
            }

            let result = SetWindowPos(
                self.border_window,
                hwnd_above_tracking.unwrap_or(HWND_TOP),
                self.window_rect.left,
                self.window_rect.top,
                self.window_rect.right - self.window_rect.left,
                self.window_rect.bottom - self.window_rect.top,
                u_flags,
            );
            if result.is_err() {
                error!("Error setting window pos!");
                let _ = ShowWindow(self.border_window, SW_HIDE);
                self.pause = true;
            }
        }
        Ok(())
    }

    pub fn update_color(&mut self) -> Result<()> {
        if WinApi::is_window_active(self.tracking_window) {
            self.current_color = self.active_color.clone()
        } else {
            self.current_color = self.inactive_color.clone()
        };

        Ok(())
    }

    pub fn render(&mut self) -> Result<()> {
        // Get the render target
        let render_target = match self.render_target.get() {
            Some(rt) => rt,
            None => return Ok(()), // Return early if there is no render target
        };

        let pixel_size = D2D_SIZE_U {
            width: (self.window_rect.right - self.window_rect.left) as u32,
            height: (self.window_rect.bottom - self.window_rect.top) as u32,
        };

        self.rounded_rect.rect = D2D_RECT_F {
            left: (self.border_size / 2 - self.border_offset) as f32,
            top: (self.border_size / 2 - self.border_offset) as f32,
            right: (self.window_rect.right - self.window_rect.left - self.border_size / 2
                + self.border_offset) as f32,
            bottom: (self.window_rect.bottom - self.window_rect.top - self.border_size / 2
                + self.border_offset) as f32,
        };

        unsafe {
            let _ = render_target.Resize(ptr::addr_of!(pixel_size));

            let now = std::time::Instant::now();
            let is_active = WinApi::is_window_active(self.tracking_window);
            let last_render_time = if is_active {
                self.last_render_time_active
            } else {
                self.last_render_time_inactive
            };
            let elapsed = now
                .duration_since(last_render_time.unwrap_or(now))
                .as_secs_f32();
            if self.use_active_animation && is_active {
                self.last_render_time_active = Some(now);
                self.active_gradient_angle += 180.0 * elapsed;
                if self.active_gradient_angle > 360.0 {
                    self.active_gradient_angle -= 360.0;
                }
            } else if self.use_inactive_animation && !is_active {
                self.last_render_time_inactive = Some(now);
                self.inactive_gradient_angle += 180.0 * elapsed;
                if self.inactive_gradient_angle > 360.0 {
                    self.inactive_gradient_angle -= 360.0;
                }
            }

            // let brush = self.create_brush(self.current_color.clone()).unwrap();

            let condition = if is_active {
                self.use_active_animation
            } else {
                self.use_inactive_animation
            };

            let gradient_angle = if is_active {
                self.active_gradient_angle
            } else {
                self.inactive_gradient_angle
            };

            let brush = Brush {
                render_target: render_target.clone(),
                color: self.current_color.clone(),
                rect: self.window_rect,
                use_animation: condition,
                brush_properties: self.brush_properties,
                gradient_angle: Some(gradient_angle)
            }.to_id2d1_brush().unwrap();

            render_target.BeginDraw();
            render_target.Clear(None);
            render_target.DrawRoundedRectangle(
                &self.rounded_rect,
                &brush,
                self.border_size as f32,
                None,
            );
            let _ = render_target.EndDraw(None, None);
            let _ = InvalidateRect(self.border_window, None, false);
        }

        Ok(())
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
                if !WinApi::has_native_border(self.tracking_window) {
                    let _ = self.update_position(Some(SWP_HIDEWINDOW));
                    return LRESULT(0);
                }

                let flags = if !WinApi::is_window_visible(self.border_window) {
                    Some(SWP_SHOWWINDOW)
                } else {
                    None
                };

                let old_rect = self.window_rect;
                let _ = self.update_window_rect();
                let _ = self.update_position(flags);

                // TODO When a window is minimized, all four points of the rect go way below 0. For
                // some reason, after unminimizing/restoring, render() will sometimes render at
                // this minimized size. self.window_rect = old_rect is hopefully only a temporary solution.
                if !WinApi::is_rect_visible(&self.window_rect) {
                    self.window_rect = old_rect;
                } else if !WinApi::are_rects_same_size(&self.window_rect, &old_rect) {
                    // Only re-render the border when its size changes
                    let _ = self.render();
                }
            }
            // EVENT_OBJECT_REORDER
            WM_APP_REORDER => {
                if self.pause {
                    return LRESULT(0);
                }

                let _ = self.update_color();
                let _ = self.update_position(None);
                let _ = self.render();
            }
            // EVENT_OBJECT_SHOW / EVENT_OBJECT_UNCLOAKED
            WM_APP_SHOWUNCLOAKED => {
                if WinApi::has_native_border(self.tracking_window) {
                    let _ = self.update_color();
                    let _ = self.update_window_rect();
                    let _ = self.update_position(Some(SWP_SHOWWINDOW));
                    let _ = self.render();
                }
                self.pause = false;
            }
            // EVENT_OBJECT_HIDE / EVENT_OBJECT_CLOAKED
            WM_APP_HIDECLOAKED => {
                let _ = self.update_position(Some(SWP_HIDEWINDOW));
                self.pause = true;
            }
            // EVENT_OBJECT_MINIMIZESTART
            WM_APP_MINIMIZESTART => {
                let _ = self.update_position(Some(SWP_HIDEWINDOW));
                self.pause = true;
            }
            // EVENT_SYSTEM_MINIMIZEEND
            // When a window is about to be unminimized, hide the border and let the thread sleep
            // for 200ms to wait for the window animation to finish, then show the border.
            WM_APP_MINIMIZEEND => {
                thread::sleep(time::Duration::from_millis(self.unminimize_delay));

                if WinApi::has_native_border(self.tracking_window) {
                    let _ = self.update_color();
                    let _ = self.update_window_rect();
                    let _ = self.update_position(Some(SWP_SHOWWINDOW));
                    let _ = self.render();
                }
                self.pause = false;
            } 
            WM_PAINT => {
                let _ = self.render();
                let _ = ValidateRect(window, None);

                if self.timer_id.is_none() {
                    let _ = SetTimer(window, 1, 16, None);
                    self.timer_id = Some(1);
                }
            }
            WM_TIMER => {
                let _ = InvalidateRect(window, None, false);
            }
            WM_DESTROY => {
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
