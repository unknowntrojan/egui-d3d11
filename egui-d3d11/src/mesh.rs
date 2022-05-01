use egui::Mesh;
use std::mem::size_of;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
    D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
};

#[inline]
pub fn create_vertex_buffer(device: &ID3D11Device, mesh: &Mesh) -> ID3D11Buffer {
    create_buffer::<0>(device, mesh)
}

#[inline]
pub fn create_index_buffer(device: &ID3D11Device, mesh: &Mesh) -> ID3D11Buffer {
    create_buffer::<1>(device, mesh)
}

fn create_buffer<const N: usize>(device: &ID3D11Device, mesh: &Mesh) -> ID3D11Buffer {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: (mesh.indices.len() * size_of::<u32>()) as u32,
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: if N == 0 {
            D3D11_BIND_VERTEX_BUFFER.0
        } else if N == 1 {
            D3D11_BIND_INDEX_BUFFER.0
        } else {
            unreachable!()
        },
        ..Default::default()
    };

    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: mesh.indices.as_ptr() as _,
        ..Default::default()
    };

    unsafe { expect!(device.CreateBuffer(&desc, &init), "Failed to create buffer") }
}
