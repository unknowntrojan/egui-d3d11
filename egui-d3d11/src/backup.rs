use std::{cell::RefCell, mem::zeroed};
use windows::Win32::{
    Foundation::RECT,
    Graphics::{
        Direct3D::D3D_PRIMITIVE_TOPOLOGY,
        Direct3D11::{
            ID3D11BlendState, ID3D11Buffer, ID3D11ClassInstance, ID3D11DepthStencilState,
            ID3D11DeviceContext, ID3D11GeometryShader, ID3D11InputLayout, ID3D11PixelShader,
            ID3D11RasterizerState, ID3D11SamplerState, ID3D11ShaderResourceView,
            ID3D11VertexShader, D3D11_COMMONSHADER_CONSTANT_BUFFER_API_SLOT_COUNT,
            D3D11_COMMONSHADER_INPUT_RESOURCE_SLOT_COUNT, D3D11_COMMONSHADER_SAMPLER_SLOT_COUNT,
            D3D11_VIEWPORT, D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE,
        },
        Dxgi::Common::DXGI_FORMAT,
    },
};

/// Structe used to backup all data from directx context.
/// Thanks ImGui.
#[derive(Default)]
pub struct BackupState(RefCell<InnerState>);

impl BackupState {
    #[inline]
    pub fn save(&self, context: &ID3D11DeviceContext) {
        unsafe {
            self.0.borrow_mut().save(context);
        }
    }

    #[inline]
    pub fn restore(&self, context: &ID3D11DeviceContext) {
        unsafe {
            self.0.borrow_mut().restore(context);
        }
    }
}

#[derive(Default)]
struct InnerState {
    scissor_rects: [RECT; D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE as _],
    scissor_count: u32,

    viewports: [D3D11_VIEWPORT; D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE as _],
    viewport_count: u32,

    raster_state: Option<ID3D11RasterizerState>,

    blend_state: Option<ID3D11BlendState>,
    blend_factor: [f32; 4],
    blend_mask: u32,

    depth_stencil_state: Option<ID3D11DepthStencilState>,
    stencil_ref: u32,

    pixel_shader_resources: Array<
        { (D3D11_COMMONSHADER_INPUT_RESOURCE_SLOT_COUNT - 1) as usize },
        ID3D11ShaderResourceView,
    >,
    samplers: Array<{ (D3D11_COMMONSHADER_SAMPLER_SLOT_COUNT - 1) as usize }, ID3D11SamplerState>,

    vertex_shader: Option<ID3D11VertexShader>,
    vertex_shader_instances: Array<256, ID3D11ClassInstance>,
    vertex_shader_instances_count: u32,

    geometry_shader: Option<ID3D11GeometryShader>,
    geometry_shader_instances: Array<256, ID3D11ClassInstance>,
    geomentry_shader_instances_count: u32,

    pixel_shader: Option<ID3D11PixelShader>,
    pixel_shader_instances: Array<256, ID3D11ClassInstance>,
    pixel_shader_instances_count: u32,

    constant_buffers:
        Array<{ (D3D11_COMMONSHADER_CONSTANT_BUFFER_API_SLOT_COUNT - 1) as usize }, ID3D11Buffer>,
    primitive_topology: D3D_PRIMITIVE_TOPOLOGY,

    index_buffer: Option<ID3D11Buffer>,
    index_buffer_format: DXGI_FORMAT,
    index_buffer_offest: u32,

    vertex_buffer: Option<ID3D11Buffer>,
    vertex_buffer_strides: u32,
    vertex_buffer_offsets: u32,

    input_layout: Option<ID3D11InputLayout>,
}

impl InnerState {
    #[inline]
    pub unsafe fn save(&mut self, ctx: &ID3D11DeviceContext) {
        ctx.RSGetScissorRects(
            &mut self.scissor_count,
            Some(self.scissor_rects.as_mut_ptr()),
        );
        ctx.RSGetViewports(&mut self.viewport_count, Some(self.viewports.as_mut_ptr()));
        self.raster_state = ctx.RSGetState().ok();
        ctx.OMGetBlendState(
            Some(&mut self.blend_state),
            Some(self.blend_factor.as_mut_ptr()),
            Some(&mut self.blend_mask),
        );
        ctx.OMGetDepthStencilState(
            Some(&mut self.depth_stencil_state),
            Some(&mut self.stencil_ref),
        );
        ctx.PSGetShaderResources(0, Some(self.pixel_shader_resources.as_mut_slice()));
        ctx.PSGetSamplers(0, Some(self.samplers.as_mut_slice()));
        self.pixel_shader_instances_count = 256;
        self.vertex_shader_instances_count = 256;
        self.geomentry_shader_instances_count = 256;

        ctx.PSGetShader(
            &mut self.pixel_shader,
            Some(self.pixel_shader_instances.as_mut_ptr()),
            Some(&mut self.pixel_shader_instances_count),
        );
        ctx.VSGetShader(
            &mut self.vertex_shader,
            Some(self.vertex_shader_instances.as_mut_ptr()),
            Some(&mut self.vertex_shader_instances_count),
        );
        ctx.GSGetShader(
            &mut self.geometry_shader,
            Some(self.geometry_shader_instances.as_mut_ptr()),
            Some(&mut self.geomentry_shader_instances_count),
        );

        ctx.VSGetConstantBuffers(0, Some(self.constant_buffers.as_mut_slice()));
        self.primitive_topology = ctx.IAGetPrimitiveTopology();
        ctx.IAGetIndexBuffer(
            Some(&mut self.index_buffer),
            Some(&mut self.index_buffer_format),
            Some(&mut self.index_buffer_offest),
        );
        ctx.IAGetVertexBuffers(
            0,
            1,
            Some(&mut self.vertex_buffer),
            Some(&mut self.vertex_buffer_strides),
            Some(&mut self.vertex_buffer_offsets),
        );
        self.input_layout = ctx.IAGetInputLayout().ok();
    }

    #[inline]
    pub unsafe fn restore(&mut self, ctx: &ID3D11DeviceContext) {
        ctx.RSSetScissorRects(Some(
            &self.scissor_rects.as_slice()[..self.scissor_count as usize],
        ));
        ctx.RSSetViewports(Some(
            &self.viewports.as_slice()[..self.viewport_count as usize],
        ));
        if let Some(raster_state) = self.raster_state.take() {
            ctx.RSSetState(&raster_state);
        }
        ctx.OMSetBlendState(
            &expect!(self.blend_state.take(), "Failed to set blend state"),
            Some(self.blend_factor.as_ptr()),
            self.blend_mask,
        );
        if let Some(depth_stencil_state) = self.depth_stencil_state.take() {
            ctx.OMSetDepthStencilState(&depth_stencil_state, self.stencil_ref);
        }
        // this is really dumb, but I couldn't find a way to make it cleaner
        // as PSGetShaderResources gives us a &[Option<ID3D11ShaderResourceView>] while
        // PSSetShaderResources wants a &[ID3D11ShaderResourceView]
        // there's multiple of these filter maps sadly
        ctx.PSSetShaderResources(
            0,
            Some(
                &self
                    .pixel_shader_resources
                    .0
                    .iter()
                    .filter_map(|x| x.clone())
                    .collect::<Vec<_>>(),
            ),
        );

        ctx.PSSetSamplers(
            0,
            Some(
                &self
                    .samplers
                    .0
                    .iter()
                    .filter_map(|x| x.clone())
                    .collect::<Vec<_>>(),
            ),
        );
        ctx.PSSetShader(
            &expect!(self.pixel_shader.take(), "Failed to set pixel shader"),
            Some(
                &self
                    .pixel_shader_instances
                    .0
                    .iter()
                    .filter_map(|x| x.clone())
                    .collect::<Vec<_>>()[..self.pixel_shader_instances_count as usize],
            ),
        );
        self.pixel_shader_instances.release();

        ctx.VSSetShader(
            &expect!(self.vertex_shader.take(), "Failed to set vertex shader"),
            Some(
                &self
                    .vertex_shader_instances
                    .0
                    .iter()
                    .filter_map(|x| x.clone())
                    .collect::<Vec<_>>()[..self.vertex_shader_instances_count as usize],
            ),
        );
        self.vertex_shader_instances.release();

        if let Some(geo_shader) = &self.geometry_shader.take() {
            ctx.GSSetShader(
                geo_shader,
                Some(
                    &self.geometry_shader_instances.0
                        [..self.geomentry_shader_instances_count as usize]
                        .iter()
                        .filter_map(|x| x.clone())
                        .collect::<Vec<_>>(),
                ),
            );
        }
        self.geometry_shader_instances.release();

        let constant_buffers = self
            .constant_buffers
            .0
            .iter()
            .filter_map(|x| x.clone())
            .collect::<Vec<_>>();
        ctx.VSSetConstantBuffers(0, Some(&constant_buffers));
        ctx.IASetPrimitiveTopology(self.primitive_topology);
        ctx.IASetIndexBuffer(
            &expect!(self.index_buffer.take(), "Failed to set index buffer"),
            self.index_buffer_format,
            self.index_buffer_offest,
        );
        ctx.IASetVertexBuffers(
            0,
            1,
            Some(&self.vertex_buffer.take()),
            Some(&self.vertex_buffer_strides),
            Some(&self.vertex_buffer_offsets),
        );

        if let Some(input_layout) = &self.input_layout.take() {
            ctx.IASetInputLayout(input_layout);
        }
    }
}

struct Array<const N: usize, T>([Option<T>; N]);
impl<const N: usize, T> Array<N, T> {
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut Option<T> {
        &mut self.0[0]
    }

    #[inline]
    pub fn as_slice(&self) -> &[Option<T>] {
        self.0.as_slice()
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Option<T>] {
        self.0.as_mut_slice()
    }

    #[inline]
    pub fn release(&mut self) {
        self.0.iter().for_each(drop);
    }
}

impl<const N: usize, T> Default for Array<N, T> {
    fn default() -> Self {
        unsafe { zeroed() }
    }
}
