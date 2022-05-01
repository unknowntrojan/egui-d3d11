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

    unsafe fn create_shader(device: &ID3D11Device, blob: &ID3DBlob) -> Self;
}

impl Shader for ID3D11VertexShader {
    const ENTRY: PSTR = p_str!("vs_main");
    const TARGET: PSTR = p_str!("vs_5_0");

    unsafe fn create_shader(device: &ID3D11Device, blob: &ID3DBlob) -> Self {
        expect!(
            device.CreateVertexShader(blob.GetBufferPointer(), blob.GetBufferSize(), None),
            "Failed to create vertex shader"
        )
    }
}

impl Shader for ID3D11PixelShader {
    const ENTRY: PSTR = p_str!("ps_main");
    const TARGET: PSTR = p_str!("ps_5_0");

    unsafe fn create_shader(device: &ID3D11Device, blob: &ID3DBlob) -> Self {
        expect!(
            device.CreatePixelShader(blob.GetBufferPointer(), blob.GetBufferSize(), None),
            "Failed to create vertex shader"
        )
    }
}

pub struct CompiledShaders {
    pub vertex: ID3D11VertexShader,
    pub pixel: ID3D11PixelShader,
    cache: ID3DBlob,
}

impl CompiledShaders {
    pub fn new(device: &ID3D11Device) -> Self {
        let (cache, vertex) = Self::compile_shader::<ID3D11VertexShader>(device);
        let (_, pixel) = Self::compile_shader::<ID3D11PixelShader>(device);

        Self {
            vertex,
            pixel,
            cache,
        }
    }

    pub fn bytecode_ptr(&self) -> *mut () {
        unsafe { self.cache.GetBufferPointer() as _ }
    }

    pub fn bytecode_len(&self) -> usize {
        unsafe { self.cache.GetBufferSize() }
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
                    S::create_shader(device, code.as_ref().unwrap()),
                )
            }
        }
    }
}
