use bevy::{
    ecs::prelude::*,
    prelude::Handle,
    render::{
        color::Color,
        render_asset::RenderAssets,
        render_graph::{Node, NodeRunError, RenderGraphContext},
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase, TrackedRenderPass},
        render_resource::CachedPipelineId,
        renderer::RenderContext,
        texture::Image,
    },
};
use wgpu::{LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor};

use crate::AnticData;
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
                            // load: LoadOp::Clear(Color::rgba(0.0, 0.0, 0.0, 1.0).into()), // TODO: clear when paused?
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
                    bevy::ecs::entity::Entity::from_raw(0),
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
                    bevy::ecs::entity::Entity::from_raw(0),
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

            let inner = collisions.data.inner.write();
            let index = inner.buffer_index;
            let buffer = &inner.buffers[index];
            // bevy::log::info!("copy texture to buffer {}", index);
            render_context.command_encoder.copy_texture_to_buffer(
                collisions.collisions_agg_texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer,
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
