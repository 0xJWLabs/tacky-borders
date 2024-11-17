use crate::animations::AnimationType;
use crate::animations::Animations;
use crate::animations::ANIM_FADE_TO_ACTIVE;
use crate::animations::ANIM_FADE_TO_INACTIVE;
use crate::animations::ANIM_NONE;
use crate::colors::adjust_gradient_stops;
use crate::colors::interpolate_d2d1_colors;
use crate::colors::interpolate_direction;
use crate::colors::Color;
use crate::colors::Gradient;
use crate::colors::Solid;
use crate::multimedia_timer::MultimediaTimer;
use crate::windowsapi::ErrorMsg;
use crate::windowsapi::WindowsApi;
use crate::windowsapi::WM_APP_EVENTANIM;
use crate::windowsapi::WM_APP_HIDECLOAKED;
use crate::windowsapi::WM_APP_LOCATIONCHANGE;
use crate::windowsapi::WM_APP_MINIMIZEEND;
use crate::windowsapi::WM_APP_MINIMIZESTART;
use crate::windowsapi::WM_APP_REORDER;
use crate::windowsapi::WM_APP_SHOWUNCLOAKED;
use crate::windowsapi::WM_APP_TIMER;

use std::ptr;
use std::sync::LazyLock;
use std::sync::OnceLock;
use std::thread;
use std::time;

use windows::core::w;
use windows::core::Result;

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
use windows::Win32::Graphics::Direct2D::Common::D2D1_GRADIENT_STOP;
use windows::Win32::Graphics::Direct2D::Common::D2D1_PIXEL_FORMAT;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U;
use windows::Win32::Graphics::Direct2D::D2D1CreateFactory;
use windows::Win32::Graphics::Direct2D::ID2D1Factory;
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
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::Win32::UI::WindowsAndMessaging::WM_CREATE;
use windows::Win32::UI::WindowsAndMessaging::WM_DESTROY;
use windows::Win32::UI::WindowsAndMessaging::WM_PAINT;
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGED;
use windows::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGING;
use windows::Win32::UI::WindowsAndMessaging::WS_DISABLED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT;
use windows::Win32::UI::WindowsAndMessaging::WS_POPUP;

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
    pub in_event_anim: i32,
    pub timer: Option<MultimediaTimer>,
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

        if !self.animations.active.is_empty() || !self.animations.inactive.is_empty() {
            let timer_duration = (1000 / self.animations.fps) as u32;
            let timer = MultimediaTimer::start(self.border_window, timer_duration);
            self.timer = Some(timer);
        }

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
            if WindowsApi::has_native_border(self.tracking_window) {
                let _ = self.update_position(Some(SWP_SHOWWINDOW));
                let _ = self.render();

                // Sometimes, it doesn't show the window at first, so we wait 5ms and update it.
                // This is very hacky and needs to be looked into. It may be related to the issue
                // detailed in update_window_rect. TODO
                thread::sleep(time::Duration::from_millis(5));
                let _ = self.update_position(Some(SWP_SHOWWINDOW));
                let _ = self.render();
            }

            let mut message = MSG::default();

            while GetMessageW(&mut message, HWND::default(), 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            debug!(
                "{}",
                format!("exiting border thread for {:?}!", self.tracking_window)
            );
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

        Ok(())
    }

    pub fn update_window_rect(&mut self) -> Result<()> {
        let _ = WindowsApi::dwm_get_window_attribute(
            self.tracking_window,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut self.window_rect,
            Some(ErrorMsg::Fn(|| {
                error!("Error getting frame rect! This is normal for apps running with elevated privileges");
                unsafe {
                    let _ = ShowWindow(self.border_window, SW_HIDE);
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
        if WindowsApi::is_window_active(self.tracking_window) {
            if self.animations.active.contains_key(&AnimationType::Fade) {
                return Ok(());
            }
            self.current_color = self.active_color.clone();
        } else {
            if self.animations.inactive.contains_key(&AnimationType::Fade) {
                return Ok(());
            }
            self.current_color = self.inactive_color.clone();
        }

        Ok(())
    }

    pub fn render(&mut self) -> Result<()> {
        self.last_render_time = Some(std::time::Instant::now());

        let Some(render_target) = self.render_target.get() else {
            return Ok(());
        };

        let pixel_size = D2D_SIZE_U {
            width: WindowsApi::get_rect_width(self.window_rect) as u32,
            height: WindowsApi::get_rect_height(self.window_rect) as u32,
        };

        self.rounded_rect.rect = D2D_RECT_F {
            left: (self.border_width / 2 - self.border_offset) as f32,
            top: (self.border_width / 2 - self.border_offset) as f32,
            right: (self.window_rect.right - self.window_rect.left - self.border_width / 2
                + self.border_offset) as f32,
            bottom: (self.window_rect.bottom - self.window_rect.top - self.border_width / 2
                + self.border_offset) as f32,
        };

        unsafe {
            let _ = render_target.Resize(&pixel_size);

            let Some(brush) = self.current_color.create_brush(
                render_target,
                &self.window_rect,
                &self.brush_properties,
            ) else {
                return Ok(());
            };

            render_target.BeginDraw();
            render_target.Clear(None);
            render_target.DrawRoundedRectangle(
                &self.rounded_rect,
                &brush,
                self.border_width as f32,
                None,
            );
            let _ = render_target.EndDraw(None, None);
            // let _ = InvalidateRect(self.border_window, None, false);
        }

        Ok(())
    }

    pub fn animate_fade(&mut self, anim_elapsed: &time::Duration, anim_speed: f32) {
        if let Color::Solid(_) = self.active_color {
            if let Color::Solid(_) = self.inactive_color {
                // If both active and inactive color are solids, use interpolate_solids
                self.interpolate_solids(anim_elapsed, anim_speed);
            }
        } else {
            self.interpolate_gradients(anim_elapsed, anim_speed);
        }
    }

    pub fn interpolate_solids(&mut self, anim_elapsed: &time::Duration, anim_speed: f32) {
        //let before = std::time::Instant::now();
        let Color::Solid(current_solid) = self.current_color.clone() else {
            println!("an interpolation function failed pattern matching");
            return;
        };
        let Color::Solid(active_solid) = self.active_color.clone() else {
            println!("an interpolation function failed pattern matching");
            return;
        };
        let Color::Solid(inactive_solid) = self.inactive_color.clone() else {
            println!("an interpolation function failed pattern matching");
            return;
        };

        let (start_color, end_color) = match self.in_event_anim {
            ANIM_FADE_TO_ACTIVE => (&inactive_solid.color, &active_solid.color),
            ANIM_FADE_TO_INACTIVE => (&active_solid.color, &inactive_solid.color),
            _ => return,
        };

        let mut finished = false;
        self.current_color = Color::Solid(Solid {
            color: interpolate_d2d1_colors(
                &current_solid.color,
                start_color,
                end_color,
                anim_elapsed.as_secs_f32(),
                anim_speed,
                &mut finished,
            ),
        });

        if finished {
            self.in_event_anim = ANIM_NONE;
        }
        //println!("time elapsed: {:?}", before.elapsed());
    }

    pub fn interpolate_gradients(&mut self, anim_elapsed: &time::Duration, anim_speed: f32) {
        //let before = time::Instant::now();
        let current_gradient = match self.current_color.clone() {
            Color::Gradient(gradient) => gradient,
            Color::Solid(solid) => {
                // If current_color is not a gradient, that means at least one of active or inactive
                // color must be solid, so only one of these if let statements should evaluate true
                let gradient = if let Color::Gradient(active_gradient) = self.active_color.clone() {
                    active_gradient
                } else if let Color::Gradient(inactive_gradient) = self.inactive_color.clone() {
                    inactive_gradient
                } else {
                    println!("an interpolation function failed pattern matching");
                    return;
                };

                // Convert current_color to a gradient
                let mut solid_as_gradient = gradient.clone();
                for i in 0..solid_as_gradient.gradient_stops.len() {
                    solid_as_gradient.gradient_stops[i].color = solid.color;
                }
                solid_as_gradient
            }
        };
        //println!("time elapsed: {:?}", before.elapsed());

        let mut all_finished = true;
        let mut gradient_stops: Vec<D2D1_GRADIENT_STOP> = Vec::new();
        let mut gradient_stops_current = current_gradient.gradient_stops.clone();

        let target_stops_len = match self.in_event_anim {
            ANIM_FADE_TO_ACTIVE => match self.active_color.clone() {
                Color::Gradient(gradient) => gradient.gradient_stops.len(),
                _ => 0,
            },
            ANIM_FADE_TO_INACTIVE => match self.inactive_color.clone() {
                Color::Gradient(gradient) => gradient.gradient_stops.len(),
                _ => 0,
            },
            _ => 0,
        };

        let mut active_colors: Color = self.active_color.clone();
        let mut inactive_colors: Color = self.inactive_color.clone();

        if target_stops_len != 0 {
            gradient_stops_current =
                adjust_gradient_stops(gradient_stops_current, target_stops_len);
            active_colors = match active_colors {
                Color::Gradient(gradient) => {
                    let gradient_stops =
                        adjust_gradient_stops(gradient.gradient_stops, target_stops_len);
                    Color::Gradient(Gradient {
                        gradient_stops,
                        direction: gradient.direction,
                    })
                }
                Color::Solid(color) => Color::Solid(color),
            };

            inactive_colors = match inactive_colors {
                Color::Gradient(gradient) => {
                    let gradient_stops =
                        adjust_gradient_stops(gradient.gradient_stops, target_stops_len);
                    Color::Gradient(Gradient {
                        gradient_stops,
                        direction: gradient.direction,
                    })
                }
                Color::Solid(color) => Color::Solid(color),
            };
        };

        for (i, _) in gradient_stops_current.iter().enumerate() {
            let mut current_finished = false;

            let active_color = match active_colors.clone() {
                Color::Gradient(gradient) => gradient.gradient_stops[i].color,
                Color::Solid(solid) => solid.color,
            };

            let inactive_color = match inactive_colors.clone() {
                Color::Gradient(gradient) => gradient.gradient_stops[i].color,
                Color::Solid(solid) => solid.color,
            };

            let (start_color, end_color) = match self.in_event_anim {
                ANIM_FADE_TO_ACTIVE => (&inactive_color, &active_color),
                ANIM_FADE_TO_INACTIVE => (&active_color, &inactive_color),
                _ => return,
            };

            let color = interpolate_d2d1_colors(
                &gradient_stops_current[i].color,
                start_color,
                end_color,
                anim_elapsed.as_secs_f32(),
                anim_speed,
                &mut current_finished,
            );

            if !current_finished {
                all_finished = false;
            }

            // TODO currently this works well because users cannot adjust the positions of the
            // gradient stops, so both inactive and active gradients will have the same positions,
            // but this might need to be interpolated if we add position configuration.
            let position = gradient_stops_current[i].position;

            let stop = D2D1_GRADIENT_STOP { color, position };
            gradient_stops.push(stop);
        }

        let mut direction = current_gradient.direction;

        // Interpolate direction if both active and inactive are gradients
        if let Color::Gradient(inactive_gradient) = self.inactive_color.clone() {
            if let Color::Gradient(active_gradient) = self.active_color.clone() {
                let (start_direction, end_direction) = match self.in_event_anim {
                    ANIM_FADE_TO_ACTIVE => {
                        (&inactive_gradient.direction, &active_gradient.direction)
                    }
                    ANIM_FADE_TO_INACTIVE => {
                        (&active_gradient.direction, &inactive_gradient.direction)
                    }
                    _ => return,
                };

                direction = interpolate_direction(
                    &direction,
                    start_direction,
                    end_direction,
                    anim_elapsed.as_secs_f32(),
                    anim_speed,
                );
            }
        }

        if all_finished {
            match self.in_event_anim {
                ANIM_FADE_TO_ACTIVE => self.current_color = self.active_color.clone(),
                ANIM_FADE_TO_INACTIVE => self.current_color = self.inactive_color.clone(),
                _ => {}
            }
            self.in_event_anim = ANIM_NONE;
        } else {
            self.current_color = Color::Gradient(Gradient {
                gradient_stops,
                direction,
            });
        }
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

                let flags = if !WindowsApi::is_window_visible(self.border_window) {
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

                let _ = self.update_color();
                let _ = self.update_position(None);
                let _ = self.render();
            }
            // EVENT_OBJECT_SHOW / EVENT_OBJECT_UNCLOAKED
            WM_APP_SHOWUNCLOAKED => {
                let old_rect = self.window_rect;

                let _ = self.update_window_rect();
                if !WindowsApi::is_rect_visible(&self.window_rect) {
                    self.window_rect = old_rect;
                    return LRESULT(0);
                }

                if WindowsApi::has_native_border(self.tracking_window) {
                    let _ = self.update_color();
                    let _ = self.update_position(Some(SWP_SHOWWINDOW));
                    let _ = self.render();
                }

                self.pause = false;
            }
            // EVENT_OBJECT_HIDE / EVENT_OBJECT_CLOAKED / EVENT_OBJECT_MINIMIZESTART
            WM_APP_HIDECLOAKED | WM_APP_MINIMIZESTART => {
                let _ = self.update_position(Some(SWP_HIDEWINDOW));
                self.pause = true;
            }
            // EVENT_SYSTEM_MINIMIZEEND
            // When a window is about to be unminimized, hide the border and let the thread sleep
            // to wait for the window animation to finish, then show the border.
            WM_APP_MINIMIZEEND => {
                thread::sleep(time::Duration::from_millis(self.unminimize_delay));

                if WindowsApi::has_native_border(self.tracking_window) {
                    let _ = self.update_color();
                    let _ = self.update_window_rect();
                    let _ = self.update_position(Some(SWP_SHOWWINDOW));
                    let _ = self.render();
                }
                self.pause = false;
            }
            WM_APP_EVENTANIM => match wparam.0 as i32 {
                ANIM_FADE_TO_ACTIVE | ANIM_FADE_TO_INACTIVE => {
                    let animations_list = if WindowsApi::is_window_active(window) {
                        self.animations.active.clone()
                    } else {
                        self.animations.inactive.clone()
                    };

                    if animations_list.contains_key(&AnimationType::Fade) {
                        self.in_event_anim = wparam.0 as i32;
                    }
                }
                _ => {}
            },
            WM_APP_TIMER => {
                if self.pause {
                    return LRESULT(0);
                }

                let animations_list = if WindowsApi::is_window_active(self.tracking_window) {
                    self.animations.active.clone()
                } else {
                    self.animations.inactive.clone()
                };

                let anim_elapsed = self
                    .last_animation_time
                    .unwrap_or(time::Instant::now())
                    .elapsed();
                let render_elapsed = self
                    .last_render_time
                    .unwrap_or(time::Instant::now())
                    .elapsed();

                self.last_animation_time = Some(time::Instant::now());

                if animations_list.is_empty() {
                    self.brush_properties.transform = Matrix3x2::identity();
                } else {
                    for (anim_type, anim_speed) in animations_list.iter() {
                        match anim_type {
                            AnimationType::Spiral => {
                                if self.spiral_anim_angle >= 360.0 {
                                    self.spiral_anim_angle -= 360.0;
                                }
                                self.spiral_anim_angle +=
                                    (anim_elapsed.as_secs_f32() * anim_speed * 2.0).min(359.0);

                                let center_x = WindowsApi::get_rect_width(self.window_rect) / 2;
                                let center_y = WindowsApi::get_rect_height(self.window_rect) / 2;
                                self.brush_properties.transform = Matrix3x2::rotation(
                                    self.spiral_anim_angle,
                                    center_x as f32,
                                    center_y as f32,
                                );
                            }
                            AnimationType::Fade => {}
                        }
                    }
                }

                match self.in_event_anim {
                    ANIM_FADE_TO_ACTIVE | ANIM_FADE_TO_INACTIVE => {
                        let anim_speed = animations_list.get(&AnimationType::Fade).unwrap();
                        self.animate_fade(&anim_elapsed, *anim_speed / 15.0);
                    }
                    _ => {}
                }

                if render_elapsed
                    >= time::Duration::from_millis((1000 / self.animations.fps) as u64)
                {
                    let _ = self.render();
                }
            }
            WM_PAINT => {
                let _ = ValidateRect(window, None);
            }
            WM_DESTROY => {
                if let Some(ref mut timer) = self.timer {
                    timer.stop();
                }
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
