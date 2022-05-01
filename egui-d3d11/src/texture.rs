use egui::{Color32, ImageData, TextureId, TexturesDelta};
use std::{collections::HashMap, mem::size_of, slice::from_raw_parts_mut};
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        ID3D11Device, ID3D11DeviceContext, ID3D11ShaderResourceView, ID3D11Texture2D,
        D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE, D3D11_MAP_WRITE_DISCARD,
        D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA,
        D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DYNAMIC,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
};

struct ManagedTexture {
    resource: ID3D11ShaderResourceView,
    texture: ID3D11Texture2D,
    pixels: Vec<Color32>,
    width: usize,
}

#[derive(Default)]
pub struct TextureAllocator {
    allocated: HashMap<TextureId, ManagedTexture>,
}

impl TextureAllocator {
    pub fn process_deltas(
        &mut self,
        dev: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
        delta: TexturesDelta,
    ) {
        for (tid, delta) in delta.set {
            if delta.is_whole() {
                self.allocate_new(dev, tid, delta.image);
            } else {
                self.update_partial(ctx, tid, delta.image, delta.pos.unwrap());
            }
        }

        for tid in delta.free {
            self.free(tid);
        }
    }

    pub fn get_by_id(&self, tid: TextureId) -> Option<ID3D11ShaderResourceView> {
        self.allocated.get(&tid).map(|t| t.resource.clone())
    }
}

impl TextureAllocator {
    fn allocate_new(&mut self, dev: &ID3D11Device, tid: TextureId, image: ImageData) {
        let tex = Self::allocate_texture(dev, image);
        self.allocated.insert(tid, tex);
    }

    fn free(&mut self, tid: TextureId) -> bool {
        self.allocated.remove(&tid).is_some()
    }

    fn update_partial(
        &mut self,
        ctx: &ID3D11DeviceContext,
        tid: TextureId,
        image: ImageData,
        [nx, ny]: [usize; 2],
    ) -> bool {
        if let Some(old) = self.allocated.get_mut(&tid) {
            let subr = unsafe {
                expect!(
                    ctx.Map(&old.texture, 0, D3D11_MAP_WRITE_DISCARD, 0),
                    "Failed to map subresource"
                )
            }
            .pData;

            match image {
                ImageData::Font(f) => unsafe {
                    let data = from_raw_parts_mut(subr as *mut Color32, old.pixels.len());
                    data.as_mut_ptr()
                        .copy_from_nonoverlapping(old.pixels.as_ptr(), old.pixels.len());

                    let new: Vec<Color32> = f
                        .pixels
                        .iter()
                        .map(|a| Color32::from_rgba_premultiplied(255, 255, 255, (a * 255.) as u8))
                        .collect();

                    for y in 0..f.height() {
                        for x in 0..f.width() {
                            let whole = (ny + y) * old.width + nx + x;
                            let frac = y * f.width() + x;
                            old.pixels[whole] = new[frac];
                            data[whole] = new[frac];
                        }
                    }
                },
                _ => unreachable!(),
            }

            unsafe {
                ctx.Unmap(&old.texture, 0);
            }

            true
        } else {
            false
        }
    }

    fn allocate_texture(dev: &ID3D11Device, image: ImageData) -> ManagedTexture {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: image.width() as _,
            Height: image.height() as _,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_SHADER_RESOURCE,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
            ..Default::default()
        };

        // rust is cringe sometimes
        let width = image.width();
        let pixels = match image {
            ImageData::Color(c) => c.pixels,
            ImageData::Font(f) => f
                .pixels
                .iter()
                .map(|a| Color32::from_rgba_premultiplied(255, 255, 255, (a * 255.) as u8))
                .collect(),
        };

        let data = D3D11_SUBRESOURCE_DATA {
            pSysMem: pixels.as_ptr() as _,
            SysMemPitch: (width * size_of::<Color32>()) as u32,
            SysMemSlicePitch: 0,
        };

        unsafe {
            let texture = expect!(
                dev.CreateTexture2D(&desc, &data),
                "Failed to create a texture"
            );

            let desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: desc.MipLevels,
                    },
                },
            };

            let resource = expect!(
                dev.CreateShaderResourceView(&texture, &desc),
                "Failed to create shader resource view"
            );

            ManagedTexture {
                width,
                resource,
                pixels,
                texture,
            }
        }
    }
}
