use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub(crate) u32);

pub(crate) struct TextureEntry {
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

pub(crate) fn load_png(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &Path,
) -> TextureEntry {
    let img = match image::open(path) {
        Ok(img) => img.into_rgba8(),
        Err(e) => {
            log::error!("Failed to load image {}: {e}", path.display());
            // Return a 1x1 magenta placeholder so we don't crash
            return create_texture_from_rgba(device, queue, &[255, 0, 255, 255], 1, 1);
        }
    };
    let (width, height) = img.dimensions();
    create_texture_from_rgba(device, queue, &img.into_raw(), width, height)
}

pub(crate) fn create_texture_from_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[u8],
    width: u32,
    height: u32,
) -> TextureEntry {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sprite_texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    TextureEntry {
        _texture: texture,
        view,
        width,
        height,
    }
}
