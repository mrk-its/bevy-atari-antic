use bevy::{
    core_pipeline::Transparent3d,
    ecs::{
        prelude::*,
        system::{lifetimeless::*, SystemParamItem},
    },
    pbr2::{DrawMesh, MeshUniform, PbrShaders, SetMeshViewBindGroup, SetTransformBindGroup},
    prelude::{AddAsset, App, Handle, Plugin},
    render2::{
        mesh::Mesh,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_component::ExtractComponentPlugin,
        render_phase::{
            AddRenderCommand, DrawFunctions, RenderCommand, RenderPhase, TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        shader::Shader,
        texture::BevyDefault,
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};
use parking_lot::RwLock;
use std::sync::Arc;
pub mod atari_data;
pub mod resources;
use wgpu::BufferDescriptor;

use resources::{AtariPalette, GTIA1Regs, GTIA2Regs, GTIA3Regs};

pub use atari_data::{AnticData, AnticDataInner, MEMORY_UNIFORM_SIZE};

use crevice::std140::{AsStd140, Std140};

// pub type Cache = HashMap<Handle<GpuAnticData>, <A as RenderAsset>::PreparedAsset>;

#[derive(Clone)]
pub struct GpuAnticData {
    _palette_buffer: Buffer,
    _buffer1: Buffer,
    _buffer2: Buffer,
    _buffer3: Buffer,
    _texture: Texture,
    _texture_view: TextureView,
    bind_group: BindGroup,
}

impl RenderAsset for AnticData {
    type ExtractedAsset = Arc<RwLock<AnticDataInner>>;
    type PreparedAsset = Arc<GpuAnticData>;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<CustomPipeline>,
        SResMut<Option<Arc<GpuAnticData>>>,
    );
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.inner.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, render_queue, custom_pipeline, cache): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let inner = extracted_asset.read();

        if cache.is_none() {
            cache.replace(Self::create_gpu_data(&render_device, &custom_pipeline));
        }

        let gpu_data = (**cache).as_ref().unwrap();

        render_queue.write_buffer(
            &gpu_data._palette_buffer,
            0,
            inner.palette.as_std140().as_bytes(),
        );

        render_queue.write_buffer(&gpu_data._buffer1, 0, inner.gtia1.as_std140().as_bytes());

        render_queue.write_buffer(&gpu_data._buffer2, 0, inner.gtia2.as_std140().as_bytes());

        render_queue.write_buffer(&gpu_data._buffer3, 0, inner.gtia3.as_std140().as_bytes());

        render_queue.write_texture(
            gpu_data._texture.as_image_copy(),
            &inner.memory,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(256 as u32).unwrap()),
                rows_per_image: None,
            },
            Extent3d {
                width: 256,
                height: 11 * 4 * 4,
                depth_or_array_layers: 1,
            },
        );

        Ok(gpu_data.clone())
    }
}

impl AnticData {
    fn create_gpu_data(
        render_device: &RenderDevice,
        custom_pipeline: &CustomPipeline,
    ) -> Arc<GpuAnticData> {
        let texture_descriptor = wgpu::TextureDescriptor {
            size: Extent3d {
                width: 256,
                height: 11 * 4 * 4,
                depth_or_array_layers: 1,
            },
            dimension: TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            label: Some("data_texture"),
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        };

        let _texture = render_device.create_texture(&texture_descriptor);
        let _texture_view = _texture.create_view(&TextureViewDescriptor::default());

        let _palette_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: AtariPalette::std140_size_static() as u64,
            mapped_at_creation: false,
        });

        let _buffer1 = render_device.create_buffer(&BufferDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: GTIA1Regs::std140_size_static() as u64,
            mapped_at_creation: false,
        });

        let _buffer2 = render_device.create_buffer(&BufferDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: GTIA2Regs::std140_size_static() as u64,
            mapped_at_creation: false,
        });

        let _buffer3 = render_device.create_buffer(&BufferDescriptor {
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: GTIA3Regs::std140_size_static() as u64,
            mapped_at_creation: false,
        });

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: _buffer1.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: _buffer2.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: _buffer3.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: _palette_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&_texture_view),
                },
            ],
            label: None,
            layout: &custom_pipeline.atari_data_layout,
        });

        Arc::new(GpuAnticData {
            _palette_buffer,
            _buffer1,
            _buffer2,
            _buffer3,
            _texture,
            _texture_view,
            bind_group,
        })
    }
}

pub struct AtariAnticPlugin;

impl Plugin for AtariAnticPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AnticData>()
            .add_plugin(ExtractComponentPlugin::<Handle<AnticData>>::default())
            .add_plugin(RenderAssetPlugin::<AnticData>::default());
        app.sub_app(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<CustomPipeline>()
            .init_resource::<Option<Arc<GpuAnticData>>>()
            .add_system_to_stage(RenderStage::Queue, queue_custom);
    }
}

pub struct CustomPipeline {
    atari_data_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let shader = Shader::from_wgsl(include_str!("antic.wgsl"));
        let shader_module = render_device.create_shader_module(&shader);

        let atari_data_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                GTIA1Regs::std140_size_static() as u64
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                GTIA2Regs::std140_size_static() as u64
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                GTIA3Regs::std140_size_static() as u64
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(
                                AtariPalette::std140_size_static() as u64
                            ),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            view_dimension: TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Uint,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
                label: Some("atari_data_layout"),
            });

        let pbr_pipeline = world.get_resource::<PbrShaders>().unwrap();

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[
                &pbr_pipeline.view_layout,
                &atari_data_layout,
                &pbr_pipeline.mesh_layout,
            ],
        });

        let pipeline = render_device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            vertex: VertexState {
                buffers: &[VertexBufferLayout {
                    array_stride: 36,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // Uv
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 12,
                            shader_location: 1,
                        },
                        // RCustom
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 20,
                            shader_location: 2,
                        },
                    ],
                }],
                module: &shader_module,
                entry_point: "vertex",
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment",
                targets: &[ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            layout: Some(&pipeline_layout),
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
        });

        CustomPipeline {
            pipeline,
            atari_data_layout,
        }
    }
}

pub fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    antic_datas: Res<RenderAssets<AnticData>>,
    material_meshes: Query<(Entity, &Handle<AnticData>, &MeshUniform), With<Handle<Mesh>>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawCustom>()
        .unwrap();

    for (view, mut transparent_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, antic_data_handle, mesh_uniform) in material_meshes.iter() {
            if antic_datas.contains_key(antic_data_handle) {
                transparent_phase.add(Transparent3d {
                    entity,
                    draw_function: draw_custom,
                    distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                });
            }
        }
    }
}

type DrawCustom = (
    SetCustomMaterialPipeline,
    SetMeshViewBindGroup<0>,
    SetTransformBindGroup<2>,
    DrawMesh,
);

struct SetCustomMaterialPipeline;
impl RenderCommand<Transparent3d> for SetCustomMaterialPipeline {
    type Param = (
        SRes<RenderAssets<AnticData>>,
        SRes<CustomPipeline>,
        SQuery<Read<Handle<AnticData>>>,
    );
    fn render<'w>(
        _view: Entity,
        item: &Transparent3d,
        (atari_data_assets, custom_pipeline, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let antic_data_handle = query.get(item.entity).unwrap();
        // let image_bind_group = image_bind_groups.into_inner().values.get(image_handle).unwrap();
        let gpu_atari_data = atari_data_assets
            .into_inner()
            .get(antic_data_handle)
            .unwrap();
        // let image_handle = image_assets.into_inner().get(image_handle).unwrap();
        pass.set_render_pipeline(&custom_pipeline.into_inner().pipeline);
        pass.set_bind_group(1, &gpu_atari_data.bind_group, &[]);
        // pass.set_bind_group(3, &gpu_atari_data.texture_bind_group, &[]);
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

#[derive(Default, Clone, Copy, Debug)]
pub struct GTIARegs {
    pub colors: [u32; 8],
    pub colors_pm: [u32; 4],
    pub hposp: [f32; 4],
    pub hposm: [f32; 4],
    pub player_size: [f32; 4],
    pub missile_size: [f32; 4],
    pub grafp: [u32; 4],
    pub prior: u32,
    pub sizem: u32,
    pub grafm: u32,
    pub _fill: u32,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vec() {
        let v: Vec<u8> = Vec::with_capacity(16);
        assert!(v.capacity() == 16);
    }
}
