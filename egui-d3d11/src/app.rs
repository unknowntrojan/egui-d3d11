use egui::Context;
use parking_lot::{Mutex, MutexGuard};
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{LPARAM, WPARAM},
        Graphics::Dxgi::IDXGISwapChain,
    },
};

use crate::{
    backup::BackupState,
    input::{InputCollector, InputResult},
};

/// Heart and soul of this integration.
/// Main methods you are going to use are:
/// * [`Self::present`] - Should be called inside of hook are before present.
/// * [`Self::resize_buffers`] - Should be called **INSTEAD** of swapchain's `ResizeBuffers`.
/// * [`Self::wnd_proc`] - Should be called on each `WndProc`.
pub struct DirectX11App<T = ()> {
    _ui: Box<dyn FnMut(&Context, &mut T) + 'static>,
    input_collector: InputCollector,
    _backup: BackupState,
    _ctx: Mutex<Context>,
    state: Mutex<T>,
}

impl<T> DirectX11App<T>
where
    T: Default,
{
    /// Creates new app with state set to default value.
    #[inline]
    pub fn new_with_default(
        ui: impl FnMut(&Context, &mut T) + 'static,
        swap_chain: &IDXGISwapChain,
    ) -> Self {
        Self::new_with_state(ui, swap_chain, T::default())
    }
}

impl<T> DirectX11App<T> {
    /// Returns lock to state of the app.
    pub fn state(&self) -> MutexGuard<T> {
        self.state.lock()
    }

    /// Creates new app with state initialized from closule call.
    #[inline]
    pub fn new_with(
        ui: impl FnMut(&Context, &mut T) + 'static,
        swap_chain: &IDXGISwapChain,
        state: impl FnOnce() -> T,
    ) -> Self {
        Self::new_with_state(ui, swap_chain, state())
    }

    /// Creates new app with explicit state value.
    pub fn new_with_state(
        ui: impl FnMut(&Context, &mut T) + 'static,
        swap_chain: &IDXGISwapChain,
        state: T,
    ) -> Self {
        unsafe {
            let hwnd = expect!(
                swap_chain.GetDesc(),
                "Failed to get swapchain's descriptor."
            )
            .OutputWindow;

            if hwnd.is_invalid() {
                if !cfg!(feature = "no-msgs") {
                    panic!("Invalid output window descriptor.");
                } else {
                    unreachable!()
                }
            }

            Self {
                input_collector: InputCollector::new(hwnd),
                _ctx: Mutex::new(Context::default()),
                _backup: BackupState::default(),
                state: Mutex::new(state),
                _ui: Box::new(ui),
            }
        }
    }

    /// Present call. Should be called once per original present call, before or inside of hook.
    pub fn present(&self, _swap_chain: &IDXGISwapChain, _sync_interval: u32, _flags: u32) {}

    /// Call when resizing buffers.
    /// Do not call the original function before it, instead call it inside of the `original` closure.
    #[allow(clippy::too_many_arguments)]
    pub fn resize_buffers(
        &self,
        _swap_chain: &IDXGISwapChain,
        _original: impl FnOnce() -> HRESULT,
    ) -> HRESULT {
        todo!()
    }

    /// Call on each `WndProc` occurence.
    /// Returns `true` if message was recognized and dispatched by input handler,
    /// `false` otherwise.
    #[inline]
    pub fn wnd_proc(&self, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> InputResult {
        self.input_collector.process(umsg, wparam.0, lparam.0)
    }
}
