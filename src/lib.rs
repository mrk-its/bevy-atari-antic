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
        texture::{BevyDefault, TextureFormatPixelInfo},
        view::ExtractedView,
        RenderApp, RenderStage,
    },
};
pub mod atari_data;
pub mod resources;

use resources::{AtariPalette, GTIA1Regs, GTIA2Regs, GTIA3Regs};

use atari_data::AtariData;

use crevice::std140::{AsStd140, Std140};

#[derive(Clone)]
pub struct GpuAtariData {
    _memory1_buffer: Buffer,

    _palette_buffer: Buffer,
    _buffer1: Buffer,
    _buffer2: Buffer,
    _buffer3: Buffer,
    // _texture: Texture,
    // _sampler: Sampler,
    // _texture_view: TextureView,
    bind_group: BindGroup,
}

impl RenderAsset for AtariData {
    type ExtractedAsset = AtariData;
    type PreparedAsset = GpuAtariData;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>, SRes<CustomPipeline>);
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (render_device, render_queue, custom_pipeline): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        // let texture_descriptor = wgpu::TextureDescriptor {
        //     size: Extent3d {
        //         width: 256,
        //         height: 11,
        //         depth_or_array_layers: 1,
        //     },
        //     dimension: TextureDimension::D2,
        //     format: wgpu::TextureFormat::Rgba32Uint,
        //     label: None,
        //     mip_level_count: 1,
        //     sample_count: 1,
        //     usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_DST,
        // };

        // let texture = render_device.create_texture(&texture_descriptor);
        // let texture_view = texture.create_view(&TextureViewDescriptor::default());
        // let sampler_descriptor = wgpu::SamplerDescriptor::default();

        // let sampler = render_device.create_sampler(&sampler_descriptor);
        let memory_data = unsafe {
            std::slice::from_raw_parts(extracted_asset.memory.as_ptr(), 16384)
        };
        // let memory_data = &[0u8; 16384];
        let memory1_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: memory_data,
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let palette_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: extracted_asset.palette.as_std140().as_bytes(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let buffer1 = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: extracted_asset.gtia1.as_std140().as_bytes(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let buffer2 = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: extracted_asset.gtia2.as_std140().as_bytes(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let buffer3 = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: extracted_asset.gtia3.as_std140().as_bytes(),
            label: None,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer1.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: buffer2.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: buffer3.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: palette_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: memory1_buffer.as_entire_binding(),
                },

                // BindGroupEntry {
                //     binding: 4,
                //     resource: BindingResource::TextureView(&texture_view),
                // },
                // BindGroupEntry {
                //     binding: 5,
                //     resource: BindingResource::Sampler(&sampler),
                // },
            ],
            label: None,
            layout: &custom_pipeline.atari_data_layout,
        });

        // let mut data = vec![0; 256 * 11 * 4 * 4];
        // data[0..extracted_asset.texture_data.len()].copy_from_slice(&extracted_asset.texture_data);

        // let format_size = texture_descriptor.format.pixel_size();
        // render_queue.write_texture(
        //     wgpu::ImageCopyTexture {
        //         texture: &texture,
        //         mip_level: 0,
        //         origin: wgpu::Origin3d::ZERO,
        //         aspect: wgpu::TextureAspect::All,
        //     },
        //     &data,
        //     wgpu::ImageDataLayout {
        //         offset: 0,
        //         bytes_per_row: Some(
        //             std::num::NonZeroU32::new(texture_descriptor.size.width * format_size as u32)
        //                 .unwrap(),
        //         ),
        //         rows_per_image: if texture_descriptor.size.depth_or_array_layers > 1 {
        //             std::num::NonZeroU32::new(texture_descriptor.size.height)
        //         } else {
        //             None
        //         },
        //     },
        //     texture_descriptor.size,
        // );

        Ok(Self::PreparedAsset {
            _memory1_buffer: memory1_buffer,

            _palette_buffer: palette_buffer,
            _buffer1: buffer1,
            _buffer2: buffer2,
            _buffer3: buffer3,
            // _texture: texture,
            // _sampler: sampler,
            // _texture_view: texture_view,
            bind_group,
        })
    }
}
pub struct AtariAnticPlugin;

impl Plugin for AtariAnticPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AtariData>()
            .add_plugin(ExtractComponentPlugin::<Handle<AtariData>>::default())
            .add_plugin(RenderAssetPlugin::<AtariData>::default());
        app.sub_app(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<CustomPipeline>()
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
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(16384 as u64),
                        },
                        count: None,
                    },

                    // BindGroupLayoutEntry {
                    //     binding: 4,
                    //     visibility: ShaderStages::FRAGMENT,
                    //     ty: BindingType::StorageTexture {
                    //         access: wgpu::StorageTextureAccess::ReadOnly,
                    //         format: wgpu::TextureFormat::Rgba32Uint,
                    //         view_dimension: TextureViewDimension::D2,
                    //     },
                    //     count: None,
                    // },
                    // BindGroupLayoutEntry {
                    //     binding: 5,
                    //     visibility: ShaderStages::FRAGMENT,
                    //     ty: BindingType::Sampler {
                    //         filtering: false,
                    //         comparison: false,
                    //     },
                    //     count: None,
                    // },
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
                //                &data_texture_layout,
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
    antic_datas: Res<RenderAssets<AtariData>>,
    material_meshes: Query<(Entity, &Handle<AtariData>, &MeshUniform), With<Handle<Mesh>>>,
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
        SRes<RenderAssets<AtariData>>,
        SRes<CustomPipeline>,
        SQuery<Read<Handle<AtariData>>>,
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
    }
}
