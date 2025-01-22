use std::mem::ManuallyDrop;

use anyhow::Context;
use windows::Win32::Foundation::FALSE;
use windows::Win32::Graphics::Direct2D::{D2D1_BITMAP_OPTIONS, D2D1_BITMAP_OPTIONS_NONE};
use windows::Win32::Graphics::Dxgi::DXGI_SWAP_CHAIN_FLAG;
use windows::Win32::Graphics::{
    Direct2D::{
        Common::{D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT},
        D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE,
        ID2D1Bitmap1, ID2D1DeviceContext7,
    },
    DirectComposition::{DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget},
    Dxgi::{
        Common::{DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
        DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_DISCARD,
        DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory7, IDXGISurface, IDXGISwapChain1,
    },
    Gdi::HMONITOR,
};

use crate::{
    app_manager::AppManager,
    windows_api::{PointerConversion, WindowsApi},
};

#[derive(Debug, Default, Clone)]
pub struct RenderResources {
    pub d2d_context: Option<ID2D1DeviceContext7>,
    pub swap_chain: Option<IDXGISwapChain1>,
    pub composition_target: Option<IDCompositionTarget>,
    pub bitmaps: Bitmaps,
}

#[derive(Debug, Default, Clone)]
pub struct Bitmaps {
    pub target_bitmap: Option<ID2D1Bitmap1>,
    pub border_bitmap: Option<ID2D1Bitmap1>,
    pub mask_bitmap: Option<ID2D1Bitmap1>,
}

impl RenderResources {
    pub fn d2d_context(&self) -> anyhow::Result<&ID2D1DeviceContext7> {
        self.d2d_context
            .as_ref()
            .context("could not get d2d_context")
    }

    pub fn swap_chain(&self) -> anyhow::Result<&IDXGISwapChain1> {
        self.swap_chain.as_ref().context("could not get swap_chain")
    }

    pub fn target_bitmap(&self) -> anyhow::Result<&ID2D1Bitmap1> {
        self.bitmaps
            .target_bitmap
            .as_ref()
            .context("could not get target_bitmap")
    }

    pub fn border_bitmap(&self) -> anyhow::Result<&ID2D1Bitmap1> {
        self.bitmaps
            .border_bitmap
            .as_ref()
            .context("could not get border_bitmap")
    }

    pub fn mask_bitmap(&self) -> anyhow::Result<&ID2D1Bitmap1> {
        self.bitmaps
            .mask_bitmap
            .as_ref()
            .context("could not get mask_bitmap")
    }

    pub fn create(
        &mut self,
        current_monitor: HMONITOR,
        border_width: i32,
        window_padding: i32,
        border_window: isize,
    ) -> anyhow::Result<()> {
        let app_manager = AppManager::get();
        let d2d_context = unsafe {
            app_manager
                .d2d_device()
                .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
        }
        .context("d2d_context")?;

        unsafe { d2d_context.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE) };

        let m_info = WindowsApi::get_monitor_info(current_monitor).context("mi")?;
        let screen_width = (m_info.rcMonitor.right - m_info.rcMonitor.left) as u32;
        let screen_height = (m_info.rcMonitor.bottom - m_info.rcMonitor.top) as u32;

        let bitmap_size = D2D_SIZE_U {
            width: screen_width + ((border_width + window_padding) * 2) as u32,
            height: screen_height + ((border_width + window_padding) * 2) as u32,
        };

        let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: bitmap_size.width,
            Height: bitmap_size.height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: FALSE,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            Flags: 0,
        };

        unsafe {
            let dxgi_adapter = app_manager
                .dxgi_device()
                .GetAdapter()
                .context("dxgi_adapter")?;
            let dxgi_factory: IDXGIFactory7 = dxgi_adapter.GetParent().context("dxgi_factory")?;

            let swap_chain = dxgi_factory
                .CreateSwapChainForComposition(app_manager.device(), &swap_chain_desc, None)
                .context("swap_chain")?;

            let d_comp_device: IDCompositionDevice =
                DCompositionCreateDevice(app_manager.dxgi_device())?;
            let d_comp_target = d_comp_device
                .CreateTargetForHwnd(border_window.as_hwnd(), true)
                .context("d_comp_target")?;
            let d_comp_visual = d_comp_device.CreateVisual().context("visual")?;

            d_comp_visual
                .SetContent(&swap_chain)
                .context("d_comp_visual.SetContent()")?;
            d_comp_target
                .SetRoot(&d_comp_visual)
                .context("d_comp_target.SetRoot()")?;
            d_comp_device.Commit().context("d_comp_device.Commit()")?;

            self.bitmaps
                .create(&d2d_context, &swap_chain, &bitmap_size)
                .context("could not create bitmaps")?;

            self.d2d_context = Some(d2d_context);
            self.swap_chain = Some(swap_chain);
            self.composition_target = Some(d_comp_target);
        }

        Ok(())
    }

    pub fn update(
        &mut self,
        current_monitor: HMONITOR,
        border_width: i32,
        window_padding: i32,
    ) -> anyhow::Result<()> {
        // Release buffer references
        self.bitmaps.target_bitmap = None;
        self.bitmaps.border_bitmap = None;
        self.bitmaps.mask_bitmap = None;

        let d2d_context = self.d2d_context()?;
        let swap_chain = self.swap_chain()?;

        unsafe { d2d_context.SetTarget(None) };

        let m_info = WindowsApi::get_monitor_info(current_monitor).context("mi")?;
        let screen_width = (m_info.rcMonitor.right - m_info.rcMonitor.left) as u32;
        let screen_height = (m_info.rcMonitor.bottom - m_info.rcMonitor.top) as u32;

        let bitmap_size = D2D_SIZE_U {
            width: screen_width + ((border_width + window_padding) * 2) as u32,
            height: screen_height + ((border_width + window_padding) * 2) as u32,
        };

        unsafe {
            swap_chain.ResizeBuffers(
                2,
                bitmap_size.width,
                bitmap_size.height,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_SWAP_CHAIN_FLAG::default(),
            )
        }
        .context("swap_chain.ResizeBuffers()")?;

        // Supposedly, cloning d2d_context or swap_chain just increases the underlying object's
        // reference count, so it's not actually cloning the object itself. Unfortunately, I need
        // to do it because Rust's borrow checker is a little stupid.
        self.bitmaps
            .create(&d2d_context.clone(), &swap_chain.clone(), &bitmap_size)
            .context("could not create bitmaps")?;

        Ok(())
    }
}

impl Bitmaps {
    fn create_bitmap_properties(extra: Option<D2D1_BITMAP_OPTIONS>) -> D2D1_BITMAP_PROPERTIES1 {
        D2D1_BITMAP_PROPERTIES1 {
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | extra.unwrap_or(D2D1_BITMAP_OPTIONS_NONE),
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            colorContext: ManuallyDrop::new(None),
        }
    }

    fn create(
        &mut self,
        d2d_context: &ID2D1DeviceContext7,
        swap_chain: &IDXGISwapChain1,
        bitmap_size: &D2D_SIZE_U,
    ) -> anyhow::Result<()> {
        let bitmap_properties =
            Self::create_bitmap_properties(Some(D2D1_BITMAP_OPTIONS_CANNOT_DRAW));

        let dxgi_back_buffer: IDXGISurface =
            unsafe { swap_chain.GetBuffer(0) }.context("dxgi_back_buffer")?;

        let target_bitmap = unsafe {
            d2d_context.CreateBitmapFromDxgiSurface(&dxgi_back_buffer, Some(&bitmap_properties))
        }
        .context("d2d_target_bitmap")?;

        unsafe { d2d_context.SetTarget(&target_bitmap) };

        // We create two bitmaps because the first (target_bitmap) cannot be used for effects
        let bitmap_properties = Self::create_bitmap_properties(None);
        let border_bitmap =
            unsafe { d2d_context.CreateBitmap(*bitmap_size, None, 0, &bitmap_properties) }
                .context("border_bitmap")?;

        let mask_bitmap =
            unsafe { d2d_context.CreateBitmap(*bitmap_size, None, 0, &bitmap_properties) }
                .context("mask_bitmap")?;

        self.target_bitmap = Some(target_bitmap);
        self.border_bitmap = Some(border_bitmap);
        self.mask_bitmap = Some(mask_bitmap);

        Ok(())
    }
}
