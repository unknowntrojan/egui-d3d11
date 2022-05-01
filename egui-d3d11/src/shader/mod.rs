use std::io::Write;

use windows::Win32::{
    Foundation::PSTR,
    Graphics::{
        Direct3D::{
            Fxc::{D3DCompile, D3DCOMPILE_DEBUG, D3DCOMPILE_ENABLE_STRICTNESS},
            ID3DBlob,
        },
        Direct3D11::{ID3D11Device, ID3D11PixelShader, ID3D11VertexShader},
    },
};

trait Shader {
    const ENTRY: PSTR;
    const TARGET: PSTR;

    unsafe fn create_shader(device: &ID3D11Device, blob: &ShaderData) -> Self;
}

impl Shader for ID3D11VertexShader {
    const ENTRY: PSTR = p_str!("vs_main");
    const TARGET: PSTR = p_str!("vs_5_0");

    unsafe fn create_shader(device: &ID3D11Device, blob: &ShaderData) -> Self {
        match blob {
            ShaderData::EmbeddedData(arr) => {
                expect!(
                    device.CreateVertexShader(arr.as_ptr() as _, arr.len() as _, None),
                    "Failed to create vertex shader"
                )
            }
            ShaderData::CompiledBlob(blob) => {
                expect!(
                    device.CreateVertexShader(blob.GetBufferPointer(), blob.GetBufferSize(), None),
                    "Failed to create vertex shader"
                )
            }
        }
    }
}

impl Shader for ID3D11PixelShader {
    const ENTRY: PSTR = p_str!("ps_main");
    const TARGET: PSTR = p_str!("ps_5_0");

    unsafe fn create_shader(device: &ID3D11Device, blob: &ShaderData) -> Self {
        match blob {
            ShaderData::EmbeddedData(arr) => {
                expect!(
                    device.CreatePixelShader(arr.as_ptr() as _, arr.len() as _, None),
                    "Failed to create vertex shader"
                )
            }
            ShaderData::CompiledBlob(blob) => {
                expect!(
                    device.CreatePixelShader(blob.GetBufferPointer(), blob.GetBufferSize(), None),
                    "Failed to create vertex shader"
                )
            }
        }
    }
}

pub enum ShaderData {
    EmbeddedData(&'static [u8]),
    CompiledBlob(ID3DBlob),
}

impl ShaderData {
    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        match self {
            ShaderData::EmbeddedData(arr) => (*arr).as_ptr() as _,
            ShaderData::CompiledBlob(b) => unsafe { b.GetBufferPointer() as _ },
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match self {
            ShaderData::EmbeddedData(arr) => (*arr).len(),
            ShaderData::CompiledBlob(b) => unsafe { b.GetBufferSize() },
        }
    }
}

pub struct CompiledShaders {
    pub vertex: ID3D11VertexShader,
    pub pixel: ID3D11PixelShader,
    cache: ShaderData,
}

impl CompiledShaders {
    pub fn new(device: &ID3D11Device) -> Self {
        if cfg!(feature = "force-compile") {
            let (vcache, vertex) = Self::compile_shader::<ID3D11VertexShader>(device);
            let (_pcache, pixel) = Self::compile_shader::<ID3D11PixelShader>(device);

            if cfg!(feature = "save-blob") {
                unsafe {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .read(true)
                        .create(true)
                        .open("vertex.bin")
                        .unwrap()
                        .write_all(std::slice::from_raw_parts(
                            vcache.GetBufferPointer() as *mut u8,
                            vcache.GetBufferSize(),
                        ))
                        .unwrap();

                    std::fs::OpenOptions::new()
                        .write(true)
                        .read(true)
                        .create(true)
                        .open("pixel.bin")
                        .unwrap()
                        .write_all(std::slice::from_raw_parts(
                            _pcache.GetBufferPointer() as *mut u8,
                            _pcache.GetBufferSize(),
                        ))
                        .unwrap();
                }
            }

            Self {
                vertex,
                pixel,
                cache: ShaderData::CompiledBlob(vcache),
            }
        } else {
            unsafe {
                let cache = ShaderData::EmbeddedData(include_bytes!("vertex.bin"));
                let vertex = ID3D11VertexShader::create_shader(device, &cache);
                let pixel = ID3D11PixelShader::create_shader(
                    device,
                    &ShaderData::EmbeddedData(include_bytes!("pixel.bin")),
                );

                Self {
                    cache,
                    vertex,
                    pixel,
                }
            }
        }
    }

    pub fn bytecode_ptr(&self) -> *mut () {
        self.cache.as_ptr()
    }

    pub fn bytecode_len(&self) -> usize {
        self.cache.len()
    }

    fn compile_shader<S: Shader>(device: &ID3D11Device) -> (ID3DBlob, S) {
        const SHADER_TEXT: &str = include_str!("shader.hlsl");

        let mut flags = D3DCOMPILE_ENABLE_STRICTNESS;
        if cfg!(debug_assertions) {
            flags |= D3DCOMPILE_DEBUG;
        }

        let mut code = None;
        let mut error = None;

        unsafe {
            if D3DCompile(
                SHADER_TEXT.as_ptr() as _,
                SHADER_TEXT.len(),
                None,
                0 as _,
                None,
                S::ENTRY,
                S::TARGET,
                flags,
                0,
                &mut code,
                &mut error,
            )
            .is_err()
            {
                if !cfg!(feature = "no-msgs") {
                    panic!(
                        "{}",
                        std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                            error.as_ref().unwrap().GetBufferPointer() as *const u8,
                            error.as_ref().unwrap().GetBufferSize(),
                        ))
                    );
                } else {
                    panic!();
                }
            } else {
                (
                    code.clone().unwrap(),
                    S::create_shader(device, &ShaderData::CompiledBlob(code.unwrap())),
                )
            }
        }
    }
}
