use std::ptr::null_mut as null;
use windows::{
    core::PCSTR,
    Win32::Graphics::{
        Direct3D::{
            Fxc::{D3DCompile, D3DCOMPILE_DEBUG, D3DCOMPILE_ENABLE_STRICTNESS},
            ID3DBlob,
        },
        Direct3D11::{ID3D11Device, ID3D11PixelShader, ID3D11VertexShader},
    },
};

trait Shader {
    const ENTRY_POINT: PCSTR;
    const TARGET: PCSTR;

    unsafe fn create(device: &ID3D11Device, blob: &ShaderData) -> Self;
}

#[allow(dead_code)]
enum ShaderData {
    CompiledBlob(ID3DBlob),
    EmbeddedData(&'static [u8]),
}

impl Shader for ID3D11VertexShader {
    const ENTRY_POINT: PCSTR = pc_str!("vs_main");
    const TARGET: PCSTR = pc_str!("vs_5_0");

    unsafe fn create(device: &ID3D11Device, blob: &ShaderData) -> Self {
        let data = match blob {
            ShaderData::CompiledBlob(b) => std::slice::from_raw_parts(b.GetBufferPointer() as *mut u8, b.GetBufferSize()),
            ShaderData::EmbeddedData(d) => *d,
        };

        expect!(
            device.CreateVertexShader(data, None),
            "Failed to create vertex shader"
        )
    }
}

impl Shader for ID3D11PixelShader {
    const ENTRY_POINT: PCSTR = pc_str!("ps_main");
    const TARGET: PCSTR = pc_str!("ps_5_0");

    unsafe fn create(device: &ID3D11Device, blob: &ShaderData) -> Self {
        let data = match blob {
            ShaderData::CompiledBlob(b) => std::slice::from_raw_parts(b.GetBufferPointer() as *mut u8, b.GetBufferSize()),
            ShaderData::EmbeddedData(d) => *d,
        };
        expect!(
            device.CreatePixelShader(data, None),
            "Failed to create pixel shader"
        )
    }
}

#[allow(dead_code)]
pub struct CompiledShaders {
    pub vertex: ID3D11VertexShader,
    pub pixel: ID3D11PixelShader,
    bytecode: ShaderData,
}

#[allow(dead_code)]
impl CompiledShaders {
    #[inline]
    pub fn vertex_bytecode(&self) -> &[u8] {
        unsafe {
            match &self.bytecode {
                ShaderData::CompiledBlob(b) => {
                    std::slice::from_raw_parts(b.GetBufferPointer() as *mut u8, b.GetBufferSize())
                }
                ShaderData::EmbeddedData(d) => *d,
            }
        }
    }

    pub fn new(device: &ID3D11Device) -> Self {
        let vblob = Self::compile_shader::<ID3D11VertexShader>();
        let pblob = Self::compile_shader::<ID3D11PixelShader>();

        let vertex = Self::create_shader::<ID3D11VertexShader>(
            device,
            &ShaderData::CompiledBlob(vblob.clone()),
        );
        let pixel = Self::create_shader::<ID3D11PixelShader>(
            device,
            &ShaderData::CompiledBlob(pblob.clone()),
        );

        Self {
            vertex,
            pixel,
            bytecode: ShaderData::CompiledBlob(vblob),
        }
    }

    fn compile_shader<S>() -> ID3DBlob
    where
        S: Shader,
    {
        const SHADER_TEXT: &str = include_str!("shader.hlsl");

        let mut flags = D3DCOMPILE_ENABLE_STRICTNESS;
        if cfg!(debug_assertions) {
            flags |= D3DCOMPILE_DEBUG;
        }

        unsafe {
            let mut blob = None;
            let mut error = None;

            if D3DCompile(
                SHADER_TEXT.as_ptr() as _,
                SHADER_TEXT.len() as _,
                PCSTR(null()),
                null(),
                None,
                S::ENTRY_POINT,
                S::TARGET,
                flags,
                0,
                &mut blob,
                &mut error,
            )
            .is_err()
            {
                if !cfg!(feature = "no-msgs") {
                    let error = std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                        error.as_ref().unwrap().GetBufferPointer() as *const u8,
                        error.as_ref().unwrap().GetBufferSize(),
                    ));

                    panic!("{}", error);
                } else {
                    unreachable!();
                }
            }

            blob.unwrap()
        }
    }

    #[inline]
    fn create_shader<S>(device: &ID3D11Device, blob: &ShaderData) -> S
    where
        S: Shader,
    {
        unsafe { S::create(device, blob) }
    }
}
