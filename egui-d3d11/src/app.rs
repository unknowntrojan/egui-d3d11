use egui::{epaint::Primitive, Context};
use parking_lot::{Mutex, MutexGuard};
use std::mem::size_of;
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{HWND, LPARAM, RECT, WPARAM},
        Graphics::{
            Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            Direct3D11::{
                ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout, ID3D11RenderTargetView,
                ID3D11Texture2D, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_BLEND_DESC,
                D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
                D3D11_BLEND_SRC_ALPHA, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_ALWAYS,
                D3D11_CULL_NONE, D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_INPUT_ELEMENT_DESC,
                D3D11_INPUT_PER_VERTEX_DATA, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
                D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_BORDER,
                D3D11_VIEWPORT,
            },
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32_UINT,
                },
                IDXGISwapChain,
            },
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
};

use crate::{
    backup::BackupState,
    input::{InputCollector, InputResult},
    mesh::{create_index_buffer, create_vertex_buffer, GpuMesh, GpuVertex},
    shader::CompiledShaders,
    texture::TextureAllocator,
};

/// Heart and soul of this integration.
/// Main methods you are going to use are:
/// * [`Self::present`] - Should be called inside of hook or before present.
/// * [`Self::resize_buffers`] - Should be called **INSTEAD** of swapchain's `ResizeBuffers`.
/// * [`Self::wnd_proc`] - Should be called on each `WndProc`.
pub struct DirectX11App<T = ()> {
    render_view: Mutex<Option<ID3D11RenderTargetView>>,
    ui: Box<dyn FnMut(&Context, &mut T) + 'static>,
    tex_alloc: Mutex<TextureAllocator>,
    input_layout: ID3D11InputLayout,
    input_collector: InputCollector,
    shaders: CompiledShaders,
    ctx: Mutex<Context>,
    backup: BackupState,
    state: Mutex<T>,
    hwnd: HWND,
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
    const INPUT_ELEMENTS_DESC: [D3D11_INPUT_ELEMENT_DESC; 3] = [
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: p_str!("POSITION"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: p_str!("TEXCOORD"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: p_str!("COLOR"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
    ];
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
            let hwnd =
                expect!(swap_chain.GetDesc(), "Failed to get swapchain's descriptor").OutputWindow;

            if hwnd.0 == -1 {
                if !cfg!(feature = "no-msgs") {
                    panic!("Invalid output window descriptor");
                } else {
                    unreachable!()
                }
            }

            let device: ID3D11Device =
                expect!(swap_chain.GetDevice(), "Failed to get swapchain's device");

            let backbuffer: ID3D11Texture2D = expect!(
                swap_chain.GetBuffer(0),
                "Failed to get swapchain's backbuffer"
            );

            let render_view = Mutex::new(Some(expect!(
                device.CreateRenderTargetView(backbuffer, 0 as _),
                "Failed to create render target view"
            )));

            let shaders = CompiledShaders::new(&device);
            let input_layout = expect!(
                device.CreateInputLayout(
                    Self::INPUT_ELEMENTS_DESC.as_ptr() as _,
                    Self::INPUT_ELEMENTS_DESC.len() as _,
                    shaders.bytecode_ptr() as _,
                    shaders.bytecode_len()
                ),
                "Failed to create input layout"
            );

            if cfg!(debug_assertions) {
                eprintln!("Initialization finished");
            }

            Self {
                tex_alloc: Mutex::new(TextureAllocator::default()),
                input_collector: InputCollector::new(hwnd),
                ctx: Mutex::new(Context::default()),
                backup: BackupState::default(),
                state: Mutex::new(state),
                ui: Box::new(ui),
                input_layout,
                render_view,
                shaders,
                hwnd,
            }
        }
    }

    /// Present call. Should be called once per original present call, before or inside of hook.
    #[allow(clippy::cast_ref_to_mut)]
    pub fn present(&self, swap_chain: &IDXGISwapChain, _sync_interval: u32, _flags: u32) {
        unsafe {
            let (dev, ctx) = &get_device_and_context(swap_chain);

            self.backup.save(ctx);

            let view_lock = &*self.render_view.lock();
            let state_lock = &mut *self.state.lock();
            let ctx_lock = &mut *self.ctx.lock();
            let tex_lock = &mut *self.tex_alloc.lock();

            let screen = self.get_screen_size();

            if cfg!(feature = "clear") {
                ctx.ClearRenderTargetView(view_lock, [0.39, 0.58, 0.92, 1.].as_ptr());
            }

            let output = ctx_lock.run(self.input_collector.collect_input(), |ctx| {
                // Dont look here, it should be fine until someone tries to do something horrible.
                (*(self.ui.as_ref() as *const _ as *mut dyn FnMut(&Context, &mut T)))(
                    ctx, state_lock,
                )
            });

            if !output.textures_delta.is_empty() {
                tex_lock.process_deltas(dev, ctx, output.textures_delta);
            }

            if output.shapes.is_empty() {
                self.backup.restore(ctx);
                return;
            }

            if !output.platform_output.copied_text.is_empty() {
                // @TODO: Paste text
            }

            let primitives = ctx_lock
                .tessellate(output.shapes)
                .into_iter()
                .filter_map(|prim| {
                    if let Primitive::Mesh(mesh) = prim.primitive {
                        GpuMesh::from_mesh(screen, mesh, prim.clip_rect)
                    } else {
                        panic!("Paint callbacks are not yet supported")
                    }
                })
                .collect::<Vec<_>>();

            self.set_blend_state(dev, ctx);
            self.set_raster_options(dev, ctx);
            self.set_sampler_state(dev, ctx);

            ctx.RSSetViewports(1, &self.get_viewport() as _);
            ctx.OMSetRenderTargets(1, view_lock, None);
            ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.IASetInputLayout(&self.input_layout);

            for mesh in primitives {
                let idx = create_index_buffer(dev, &mesh);
                let vtx = create_vertex_buffer(dev, &mesh);

                let texture = tex_lock.get_by_id(mesh.texture_id);

                ctx.RSSetScissorRects(
                    1,
                    &RECT {
                        left: mesh.clip.left() as _,
                        top: mesh.clip.top() as _,
                        right: mesh.clip.right() as _,
                        bottom: mesh.clip.bottom() as _,
                    },
                );

                if texture.is_some() {
                    ctx.PSSetShaderResources(0, 1, &texture);
                }

                ctx.IASetVertexBuffers(0, 1, &Some(vtx), &(size_of::<GpuVertex>() as _), &0);
                ctx.IASetIndexBuffer(idx, DXGI_FORMAT_R32_UINT, 0);
                ctx.VSSetShader(&self.shaders.vertex, &None, 0);
                ctx.PSSetShader(&self.shaders.pixel, &None, 0);

                ctx.DrawIndexed(mesh.indices.len() as _, 0, 0);
            }

            self.backup.restore(ctx);
        }
    }

    /// Call when resizing buffers.
    /// Do not call the original function before it, instead call it inside of the `original` closure.
    /// # Behavior
    /// In `origin` closure make sure to call the original `ResizeBuffers`.
    pub fn resize_buffers(
        &self,
        swap_chain: &IDXGISwapChain,
        original: impl FnOnce() -> HRESULT,
    ) -> HRESULT {
        unsafe {
            let view_lock = &mut *self.render_view.lock();
            std::ptr::drop_in_place(view_lock);

            let result = original();

            let backbuffer: ID3D11Texture2D = expect!(
                swap_chain.GetBuffer(0),
                "Failed to get swapchain's backbuffer"
            );

            let device: ID3D11Device =
                expect!(swap_chain.GetDevice(), "Failed to get swapchain's device");

            let new_view = expect!(
                device.CreateRenderTargetView(backbuffer, 0 as _),
                "Failed to create render target view"
            );

            *view_lock = Some(new_view);
            result
        }
    }

    /// Call on each `WndProc` occurence.
    /// Returns `true` if message was recognized and dispatched by input handler,
    /// `false` otherwise.
    #[inline]
    pub fn wnd_proc(&self, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> InputResult {
        self.input_collector.process(umsg, wparam.0, lparam.0)
    }
}

impl<T> DirectX11App<T> {
    #[inline]
    fn get_screen_size(&self) -> (f32, f32) {
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect);
        }
        (
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
        )
    }

    #[inline]
    fn get_viewport(&self) -> D3D11_VIEWPORT {
        let (w, h) = self.get_screen_size();
        D3D11_VIEWPORT {
            TopLeftX: 0.,
            TopLeftY: 0.,
            Width: w,
            Height: h,
            MinDepth: 0.,
            MaxDepth: 1.,
        }
    }

    fn set_blend_state(&self, dev: &ID3D11Device, ctx: &ID3D11DeviceContext) {
        let mut targets: [D3D11_RENDER_TARGET_BLEND_DESC; 8] = Default::default();
        targets[0].BlendEnable = true.into();
        targets[0].SrcBlend = D3D11_BLEND_SRC_ALPHA;
        targets[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
        targets[0].BlendOp = D3D11_BLEND_OP_ADD;
        targets[0].SrcBlendAlpha = D3D11_BLEND_ONE;
        targets[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;
        targets[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
        targets[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL.0 as _;

        let blend_desc = D3D11_BLEND_DESC {
            AlphaToCoverageEnable: false.into(),
            IndependentBlendEnable: false.into(),
            RenderTarget: targets,
        };

        unsafe {
            let blend_state = expect!(
                dev.CreateBlendState(&blend_desc),
                "Failed to create blend state"
            );
            ctx.OMSetBlendState(&blend_state, [0., 0., 0., 0.].as_ptr(), 0xffffffff);
        }
    }

    fn set_raster_options(&self, dev: &ID3D11Device, ctx: &ID3D11DeviceContext) {
        let mut raster = None;
        let mut desc = D3D11_RASTERIZER_DESC::default();

        unsafe {
            ctx.RSGetState(&mut raster);
            raster.unwrap().GetDesc(&mut desc);
        }

        desc.CullMode = D3D11_CULL_NONE;
        desc.ScissorEnable = true.into();

        unsafe {
            let options = expect!(
                dev.CreateRasterizerState(&desc),
                "Failed to create rasterizer state"
            );
            ctx.RSSetState(&options);
        }
    }

    fn set_sampler_state(&self, dev: &ID3D11Device, ctx: &ID3D11DeviceContext) {
        let desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_BORDER,
            AddressV: D3D11_TEXTURE_ADDRESS_BORDER,
            AddressW: D3D11_TEXTURE_ADDRESS_BORDER,
            MipLODBias: 0.,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            MinLOD: 0.,
            MaxLOD: 0.,
            BorderColor: [1., 1., 1., 1.],
            ..Default::default()
        };

        unsafe {
            let sampler = expect!(dev.CreateSamplerState(&desc), "Failed to create sampler");
            ctx.PSSetSamplers(0, 1, &Some(sampler));
        }
    }
}

unsafe fn get_device_and_context(swap: &IDXGISwapChain) -> (ID3D11Device, ID3D11DeviceContext) {
    let device: ID3D11Device = expect!(swap.GetDevice(), "Failed to get swapchain's device");
    let mut ctx = None;
    device.GetImmediateContext(&mut ctx);
    (
        device,
        expect!(ctx, "Failed to get device's immediate context"),
    )
}
