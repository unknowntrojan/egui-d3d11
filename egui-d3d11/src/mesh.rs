use egui::{Mesh, epaint::Vertex};
use std::mem::size_of;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
    D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
};

pub fn normalize_mesh((w, h): (f32, f32), m: &mut Mesh) {
    for v in m.vertices.iter_mut() {
        v.pos.x -= w / 2.;
        v.pos.y -= h / 2.;

        v.pos.x /= w / 2.;
        v.pos.y /= -h / 2.;
    }
}

pub fn create_vertex_buffer(device: &ID3D11Device, mesh: &Mesh) -> ID3D11Buffer {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: (mesh.vertices.len() * size_of::<Vertex>()) as u32,
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_VERTEX_BUFFER.0,
        ..Default::default()
    };

    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: mesh.indices.as_ptr() as _,
        ..Default::default()
    };

    unsafe { expect!(device.CreateBuffer(&desc, &init), "Failed to create vertex buffer") }
}

pub fn create_index_buffer(device: &ID3D11Device, mesh: &Mesh) -> ID3D11Buffer {
    let desc = D3D11_BUFFER_DESC {
        ByteWidth: (mesh.indices.len() * size_of::<u32>()) as u32,
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_INDEX_BUFFER.0,
        ..Default::default()
    };

    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: mesh.indices.as_ptr() as _,
        ..Default::default()
    };

    unsafe { expect!(device.CreateBuffer(&desc, &init), "Failed to create index buffer") }
}
