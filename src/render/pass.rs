use futures_lite::future;

use bevy::{
    ecs::prelude::*,
    prelude::Handle,
    render2::{
        color::Color,
        render_asset::RenderAssets,
        render_graph::{
            Node, NodeRunError, RenderGraphContext,
        },
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase, TrackedRenderPass},
        render_resource::CachedPipelineId,
        renderer::RenderContext,
        texture::Image,
    },
};
use wgpu::{Buffer, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor};

use crate::{AnticData, CollisionsData};
pub struct AnticPhase {
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub antic_data_handle: Handle<AnticData>,
    pub main_image_handle: Handle<Image>,
    pub collisions: bool,
}
pub struct CollisionsAggPhase {
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub antic_data_handle: Handle<AnticData>,
}

impl PhaseItem for AnticPhase {
    type SortKey = Entity;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl PhaseItem for CollisionsAggPhase {
    type SortKey = Entity;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

#[derive(Default)]
pub struct AnticPassNode;

impl Node for AnticPassNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let antic_data_assets = world.get_resource::<RenderAssets<AnticData>>().unwrap();
        let image_assets = world.get_resource::<RenderAssets<Image>>().unwrap();

        let render_phase = world.get_resource::<RenderPhase<AnticPhase>>().unwrap();
        for item in render_phase.items.iter() {
            let main_texture = if let Some(texture) = image_assets.get(&item.main_image_handle) {
                &texture.texture_view
            } else {
                continue;
            };

            let color_attachments = if item.collisions {
                let gpu_antic_data = antic_data_assets.get(&item.antic_data_handle).unwrap();
                let collisions_texture = &gpu_antic_data
                    .inner
                    .collisions
                    .as_ref()
                    .unwrap()
                    .collisions_texture_view;
                vec![
                    RenderPassColorAttachment {
                        view: main_texture,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    },
                    RenderPassColorAttachment {
                        view: collisions_texture,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    },
                ]
            } else {
                vec![RenderPassColorAttachment {
                    view: main_texture,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                }]
            };
            let pass_descriptor = RenderPassDescriptor {
                label: Some("antic_main_pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: None,
            };

            {
                let draw_functions = world.get_resource::<DrawFunctions<AnticPhase>>().unwrap();

                let render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&pass_descriptor);
                let mut draw_functions = draw_functions.write();

                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(
                    world,
                    &mut tracked_pass,
                    bevy::ecs::entity::Entity::new(0),
                    item,
                );
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct CollisionsAggNode;

impl Node for CollisionsAggNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // let collisions_agg_texture = graph.get_input_texture("collisions_agg_texture_view")?;
        let antic_data_assets = world.get_resource::<RenderAssets<AnticData>>().unwrap();

        let _clear_color = Color::rgba(0.0, 0.0, 0.0, 0.0);

        let collisions_agg_render_phase = world
            .get_resource::<RenderPhase<CollisionsAggPhase>>()
            .unwrap();
        for item in collisions_agg_render_phase.items.iter() {
            let gpu_antic_data = antic_data_assets.get(&item.antic_data_handle).unwrap();
            let collisions_data = gpu_antic_data.inner.collisions.as_ref().unwrap();

            let collisions_agg_pass_descriptor = RenderPassDescriptor {
                label: Some("collisioons_agg_pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &collisions_data.collisions_agg_texture_view,
                    resolve_target: None,
                    ops: Operations {
                        // load: LoadOp::Clear(clear_color.into()), // TODO: do not clear?
                        load: LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            };

            {
                let draw_functions = world
                    .get_resource::<DrawFunctions<CollisionsAggPhase>>()
                    .unwrap();

                let render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&collisions_agg_pass_descriptor);
                let mut draw_functions = draw_functions.write();

                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(
                    world,
                    &mut tracked_pass,
                    bevy::ecs::entity::Entity::new(0),
                    item,
                );
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct CollisionsAggReadNode;

impl Node for CollisionsAggReadNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let assets = world.get_resource::<RenderAssets<AnticData>>().unwrap();
        let collisions_agg_render_phase = world
            .get_resource::<RenderPhase<CollisionsAggPhase>>()
            .unwrap();
        for item in collisions_agg_render_phase.items.iter() {
            let gpu_antic_data = if let Some(antic_data) = assets.get(&item.antic_data_handle) {
                antic_data
            } else {
                return Ok(());
            };

            let collisions = gpu_antic_data.inner.collisions.as_ref().unwrap();
            let copy_size = crate::COLLISIONS_AGG_TEXTURE_SIZE;

            self.read_collisions(&collisions.buffer, collisions.data.clone(), render_context);

            // consider moving this reading befor emulation step
            // for now we have additional 1 frame delay
            render_context.command_encoder.copy_texture_to_buffer(
                collisions.collisions_agg_texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &collisions.buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(
                                copy_size.width * crate::COLLISIONS_AGG__BYTES_PER_PIXEL as u32,
                            )
                            .unwrap(),
                        ),
                        rows_per_image: None,
                    },
                },
                copy_size,
            );
        }
        Ok(())
    }
}

impl CollisionsAggReadNode {
    fn read_collisions(
        &self,
        buffer: &Buffer,
        collisions_data: CollisionsData,
        render_context: &RenderContext,
    ) {
        let slice = buffer.slice(..);
        let map_future = slice.map_async(wgpu::MapMode::Read);
        render_context.render_device.poll(wgpu::Maintain::Wait);
        future::block_on(map_future).unwrap();
        {
            let buffer_view = slice.get_mapped_range();
            let data: &[u8] = &buffer_view;
            let data =
                unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u64, data.len() / 8) };
            // bevy::log::info!("data: {:x?}", data);
            let guard = &mut collisions_data.write();
            let dest = guard.as_mut();

            for y in 0..crate::COLLISIONS_AGG_TEXTURE_SIZE.height as usize {
                if y == 0 {
                    for x in 0..240 {
                        dest[x] = data[y << 8 | x];
                    }
                } else {
                    for x in 0..240 {
                        dest[x] |= data[y << 8 | x];
                    }
                }
            }
        }
        buffer.unmap();
    }
}
