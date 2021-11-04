use bevy::{
    ecs::prelude::*,
    prelude::Handle,
    render2::{
        color::Color,
        render_asset::RenderAssets,
        render_graph::Node,
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase, TrackedRenderPass},
        render_resource::{CachedPipelineId, Texture, TextureView},
        texture::Image,
    },
};
use wgpu::{Extent3d, LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor};

use crate::{AnticData, CollisionsData, ANTIC_COLLISIONS_HANDLE, ANTIC_IMAGE_HANDLE};
pub struct AnticPhase {
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

pub struct AnticPassNode {
    texture: Option<Texture>,
    texture_view: Option<TextureView>,
    collisions_texture: Option<Texture>,
    collisions_texture_view: Option<TextureView>,
    collisions_data: CollisionsData,
}

impl AnticPassNode {
    pub fn new(collisions_data: CollisionsData) -> Self {
        Self {
            texture: None,
            texture_view: None,
            collisions_texture: None,
            collisions_texture_view: None,
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

        let clear_color = Color::rgba(0.0, 0.0, 0.0, 0.0);
        let render_phase = world.get_resource::<RenderPhase<AnticPhase>>().unwrap();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("main_pass_3d"),
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
        {
            let draw_functions = world.get_resource::<DrawFunctions<AnticPhase>>().unwrap();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
            let mut draw_functions = draw_functions.write();

            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            // let antic_phase = self.query.get(world, entity)
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
        for item in render_phase.items.iter() {
            render_context.command_encoder.copy_texture_to_buffer(
                collisions_texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &self.collisions_data.buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(std::num::NonZeroU32::new(384 * 8).unwrap()),
                        rows_per_image: None,
                    },
                },
                Extent3d {
                    width: 384,
                    height: 240,
                    depth_or_array_layers: 1,
                },
            );
            self.collisions_data.read_collisions(&render_context.render_device)
        }
        Ok(())
    }
}
