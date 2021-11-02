pub mod render;
use bevy::prelude::HandleUntyped;
use bevy::reflect::TypeUuid;

use bevy::render2::render_graph::RenderGraph;
use bevy::render2::render_phase::{DrawFunctions, RenderPhase};
use bevy::{
    core_pipeline::Transparent3d,
    prelude::{info, AddAsset, App, Assets, Handle, Plugin},
    render2::{
        camera::{CameraProjection, OrthographicProjection},
        render_asset::RenderAssetPlugin,
        render_component::ExtractComponentPlugin,
        render_phase::AddRenderCommand,
        render_resource::*,
        texture::Image,
        RenderApp, RenderStage,
    },
};

pub const ANTIC_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4805239651767799999);

pub const ANTIC_IMAGE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Image::TYPE_UUID, 4805239651767799988);
pub const ANTIC_COLLISIONS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Image::TYPE_UUID, 4805239651767799989);

pub mod atari_data;
pub mod resources;
use render::pass::{AnticPassNode, AnticPhase};

pub use atari_data::{AnticData, AnticDataInner};

pub struct AtariAnticPlugin;

impl Plugin for AtariAnticPlugin {
    fn build(&self, app: &mut App) {
        let mut projection = OrthographicProjection::default();
        projection.update(384.0, 240.0);
        let projection_matrix = projection.get_projection_matrix();
        info!("projection matrix: {:?}", projection_matrix);

        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let antic_shader = Shader::from_wgsl(include_str!("render/antic.wgsl"));
        shaders.set_untracked(ANTIC_SHADER_HANDLE, antic_shader);

        app.add_asset::<AnticData>()
            // .add_asset::<AnticMesh>()
            .add_plugin(ExtractComponentPlugin::<Handle<AnticData>>::default())
            .add_plugin(RenderAssetPlugin::<AnticData>::default());

        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<DrawFunctions<AnticPhase>>()
            .init_resource::<RenderPhase<AnticPhase>>()
            .init_resource::<render::AnticPipeline>()
            .init_resource::<Option<render::GpuAnticData>>()
            .init_resource::<SpecializedPipelines<render::AnticPipeline>>()
            .add_render_command::<AnticPhase, render::SetAnticPipeline>()
            .add_system_to_stage(RenderStage::Queue, render::queue_meshes);

        let antic_node = AnticPassNode::new(&mut render_app.world);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node("antic_node", antic_node);
        graph
            .add_node_edge(
                "antic_node",
                bevy::core_pipeline::node::MAIN_PASS_DEPENDENCIES,
            )
            .unwrap();
        let mut image = Image::new(
            Extent3d {
                width: 384,
                height: 240,
                depth_or_array_layers: 1,
            },
            wgpu::TextureDimension::D2,
            vec![0; 384 * 240 * 4],
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        image.texture_descriptor.usage = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST;

        let mut images = app.world.get_resource_mut::<Assets<Image>>().unwrap();
        images.set_untracked(ANTIC_IMAGE_HANDLE, image);

        let mut collisions_image = Image::new(
            Extent3d {
                width: 384,
                height: 240,
                depth_or_array_layers: 1,
            },
            wgpu::TextureDimension::D2,
            vec![0; 384 * 240 * 4 * 2],
            wgpu::TextureFormat::Rg32Uint,
        );
        collisions_image.texture_descriptor.usage = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST;

        images.set_untracked(ANTIC_COLLISIONS_HANDLE, collisions_image);
    }
}

#[derive(Debug)]
pub struct ModeLineDescr {
    pub mode: u8,
    pub scan_line: usize,
    pub width: usize,
    pub height: usize,
    pub n_bytes: usize,
    pub line_voffset: usize,
    pub data_offset: usize,
    pub chbase: u8,
    pub pmbase: u8,
    pub hscrol: u8,
    pub video_memory_offset: usize,
    pub charset_memory_offset: usize,
}

impl ModeLineDescr {
    pub fn next_mode_line(&self) -> usize {
        return self.scan_line + self.height;
    }
    pub fn charset_size(&self) -> usize {
        match self.mode {
            2..=5 => 1024,
            6..=7 => 512,
            _ => 0,
        }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct GTIARegs {
    pub hposp: [u8; 4],
    pub hposm: [u8; 4],
    pub sizep: [u8; 4],
    pub sizem: u8,
    pub grafp: [u8; 4],
    pub grafm: u8,
    pub col: [u8; 9],
    pub prior: u8,
    pub vdelay: u8,
    pub gractl: u8,
    pub hitclr: u8,
    pub consol: u8,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vec() {
        let v: Vec<u8> = Vec::with_capacity(16);
        assert!(v.capacity() == 16);
    }
}
