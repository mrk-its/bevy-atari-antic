pub mod render;

use bevy::prelude::HandleUntyped;
use bevy::reflect::TypeUuid;

use bevy::{
    core_pipeline::Transparent3d,
    prelude::{info, AddAsset, App, Assets, Handle, Plugin},
    render2::{
        camera::{CameraProjection, OrthographicProjection},
        render_asset::RenderAssetPlugin,
        render_component::ExtractComponentPlugin,
        render_phase::AddRenderCommand,
        render_resource::*,
        RenderApp, RenderStage,
    },
};

pub const ANTIC_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4805239651767799999);

pub mod atari_data;
pub mod resources;

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
        app.sub_app(RenderApp)
            .add_render_command::<Transparent3d, render::SetAnticPipeline>()
            .init_resource::<render::AnticPipeline>()
            .init_resource::<Option<render::GpuAnticData>>()
            .init_resource::<SpecializedPipelines<render::AnticPipeline>>()
            .add_system_to_stage(RenderStage::Queue, render::queue_meshes);
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
