use bevy::{ecs::prelude::*, prelude::Handle, render2::{color::Color, render_asset::RenderAssets, render_graph::Node, render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase, TrackedRenderPass}, render_resource::{CachedPipelineId, Texture, TextureView}, renderer::RenderDevice, texture::Image}};
use wgpu::{BindGroupDescriptor, Extent3d, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor};

use crate::{AnticData, CollisionsData, ANTIC_COLLISIONS_HANDLE, ANTIC_COLLISIONS_AGG_HANDLE, ANTIC_IMAGE_HANDLE};
pub struct AnticPhase {
    pub pipeline: CachedPipelineId,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
    pub antic_data_handle: Handle<AnticData>,
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

pub struct AnticPassNode {
    texture: Option<Texture>,
    texture_view: Option<TextureView>,
    collisions_texture: Option<Texture>,
    collisions_texture_view: Option<TextureView>,
    collisions_agg_texture: Option<Texture>,
    collisions_agg_texture_view: Option<TextureView>,
    collisions_data: CollisionsData,
}

impl AnticPassNode {
    pub fn new(collisions_data: CollisionsData) -> Self {
        Self {
            texture: None,
            texture_view: None,
            collisions_texture: None,
            collisions_texture_view: None,
            collisions_agg_texture: None,
            collisions_agg_texture_view: None,
            collisions_data,
        }
    }
}

impl Node for AnticPassNode {
    fn input(&self) -> Vec<bevy::render2::render_graph::SlotInfo> {
        Vec::new()
    }

    fn output(&self) -> Vec<bevy::render2::render_graph::SlotInfo> {
        Vec::new()
    }

    fn update(&mut self, world: &mut World) {
        let render_images = world.get_resource::<RenderAssets<Image>>().unwrap();
        if let (None, Some(image)) = (
            &self.texture_view,
            render_images.get(&ANTIC_IMAGE_HANDLE.typed()),
        ) {
            self.texture_view = Some(image.texture_view.clone());
            self.texture = Some(image.texture.clone());
        }

        if let (None, Some(image)) = (
            &self.collisions_texture_view,
            render_images.get(&ANTIC_COLLISIONS_HANDLE.typed()),
        ) {
            self.collisions_texture_view = Some(image.texture_view.clone());
            self.collisions_texture = Some(image.texture.clone());
        }

        if let (None, Some(image)) = (
            &self.collisions_agg_texture_view,
            render_images.get(&ANTIC_COLLISIONS_AGG_HANDLE.typed()),
        ) {
            self.collisions_agg_texture_view = Some(image.texture_view.clone());
            self.collisions_agg_texture = Some(image.texture.clone());
        }
    }

    fn run(
        &self,
        _graph: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render2::render_graph::NodeRunError> {
        let (texture_view, collisions_texture_view) =
            match (&self.texture_view, &self.collisions_texture_view) {
                (Some(texture_view), Some(collisions_texture_view)) => {
                    (texture_view, collisions_texture_view)
                }
                _ => return Ok(()),
            };

        let collisions_texture = match &self.collisions_texture {
            Some(collisions_texture) => collisions_texture,
            _ => return Ok(()),
        };

        let collisions_agg_texture = match &self.collisions_agg_texture {
            Some(texture) => texture,
            _ => return Ok(()),
        };

        let collisions_agg_texture_view = match &self.collisions_agg_texture_view {
            Some(view) => view,
            _ => return Ok(()),
        };

        let clear_color = Color::rgba(0.0, 0.0, 0.0, 0.0);
        let render_phase = world.get_resource::<RenderPhase<AnticPhase>>().unwrap();
        let collisions_agg_render_phase = world.get_resource::<RenderPhase<CollisionsAggPhase>>().unwrap();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("antic_main_pass"),
            color_attachments: &[
                RenderPassColorAttachment {
                    view: texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color.into()),
                        store: true,
                    },
                },
                RenderPassColorAttachment {
                    view: collisions_texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color.into()),
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: None,
        };
        let collisions_agg_pass_descriptor = RenderPassDescriptor {
            label: Some("collisioons_agg_pass"),
            color_attachments: &[
                RenderPassColorAttachment {
                    view: collisions_agg_texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(clear_color.into()),
                        store: true,
                    },
                },
            ],
            depth_stencil_attachment: None,
        };

        {
            let draw_functions = world.get_resource::<DrawFunctions<AnticPhase>>().unwrap();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut draw_functions = draw_functions.write();

            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            for item in render_phase.items.iter() {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(
                    world,
                    &mut tracked_pass,
                    bevy::ecs::entity::Entity::new(0),
                    item,
                );
            }
        }

        {
            let draw_functions = world.get_resource::<DrawFunctions<CollisionsAggPhase>>().unwrap();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&collisions_agg_pass_descriptor);
            let mut draw_functions = draw_functions.write();

            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            for item in collisions_agg_render_phase.items.iter() {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(
                    world,
                    &mut tracked_pass,
                    bevy::ecs::entity::Entity::new(0),
                    item,
                );
            }
        }

        // for _item in render_phase.items.iter() {
        //     render_context.command_encoder.copy_texture_to_buffer(
        //         collisions_texture.as_image_copy(),
        //         wgpu::ImageCopyBuffer {
        //             buffer: &self.collisions_data.buffer,
        //             layout: wgpu::ImageDataLayout {
        //                 offset: 0,
        //                 bytes_per_row: Some(std::num::NonZeroU32::new(384 * 8).unwrap()),
        //                 rows_per_image: None,
        //             },
        //         },
        //         Extent3d {
        //             width: 384,
        //             height: 240,
        //             depth_or_array_layers: 1,
        //         },
        //     );
        //     self.collisions_data.read_collisions(&render_context.render_device)
        // }
        for _item in render_phase.items.iter() {
            render_context.command_encoder.copy_texture_to_buffer(
                collisions_agg_texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &self.collisions_data.buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(std::num::NonZeroU32::new(super::COLLISIONS_AGG_TEXTURE_SIZE.width * 8).unwrap()),
                        rows_per_image: None,
                    },
                },
                super::COLLISIONS_AGG_TEXTURE_SIZE,
            );
            self.collisions_data.read_collisions(&render_context.render_device)
        }
        Ok(())
    }
}
