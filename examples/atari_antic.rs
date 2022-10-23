use bevy::{
    app::AppExit,
    ecs::prelude::*,
    math::Vec3,
    prelude::{App, Assets, Handle, Transform, ClearColor, Camera2dBundle},
    render::{
        color::Color, renderer::RenderDevice, texture::Image,
        view::Msaa,
    },
    window::WindowDescriptor,
    DefaultPlugins,
};
use bevy_atari_antic::{AnticData, GTIARegs};
use bevy_atari_antic::{AtariAnticPlugin, ModeLineDescr};

use bevy::sprite::{SpriteBundle, Sprite};

#[derive(Debug)]
pub struct MemOffsets(pub [usize; 24]);

#[allow(dead_code)]
fn quit_after_few_frames(mut cnt: Local<u32>, mut app_exit_events: EventWriter<AppExit>) {
    *cnt += 1;
    if *cnt > 5 {
        app_exit_events.send(AppExit);
    }
}

fn update(
    mut atari_data_assets: ResMut<Assets<AnticData>>,
    render_device: Res<RenderDevice>,
    query: Query<&Handle<AnticData>>,
    scr_offsets: Res<MemOffsets>,
) {
    let span = bevy::utils::tracing::span!(bevy::utils::tracing::Level::INFO, "my_span");
    let _entered = span.enter();

    for handle in query.iter() {
        if let Some(atari_data) = atari_data_assets.get_mut(handle) {
            if let Some(collision_data) = atari_data.collisions_data.as_ref() {
                collision_data.read_collisions(&render_device);
            }
            let col_agg = atari_data
                .collisions_data
                .as_ref()
                .map(|cd| {
                    let guard = cd.inner.read();
                    guard.data.iter().cloned().reduce(|a, v| a | v).unwrap()
                })
                .unwrap_or(0);

            let mut inner = atari_data.inner.write();
            let c = &mut inner.memory[32 * 240 + 1024];
            *c = c.wrapping_add(1);
            let c = &mut inner.memory[32 * 240 + 1024 + 31];
            *c = c.wrapping_add(1);

            let text = format!("collisions: {:x}", col_agg);
            text.as_bytes()
                .iter()
                .cloned()
                .map(internal_chr)
                .enumerate()
                .for_each(|(i, c)| {
                    inner.memory[32 * 240 + scr_offsets.0[1] + i + 2] = c;
                });
        }
    }
}

fn internal_chr(c: u8) -> u8 {
    match c {
        0..=31 => c + 64,
        32..=95 => c - 32,
        96..=127 => c,
        _ => 128 + internal_chr(c - 128),
    }
}

fn setup(
    mut commands: Commands,
    mut antic_data_assets: ResMut<Assets<AnticData>>,
    render_device: Res<RenderDevice>,
    mut images: ResMut<Assets<Image>>,
    mut scr_offsets: ResMut<MemOffsets>,
) {
    let main_image_handle = bevy_atari_antic::create_main_image(&mut *images);
    let mut antic_data = AnticData::new(&render_device, main_image_handle.clone(), COLLISIONS);

    let coffs = antic_data.reserve_antic_memory(1024, &mut |data| {
        data.copy_from_slice(include_bytes!("charset.dat"))
    });

    let voffs = antic_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });
    scr_offsets.0[0] = voffs;
    let start_scan_line = 104;

    let voffs0 = voffs;
    antic_data.insert_mode_line(&ModeLineDescr {
        mode: 0,
        scan_line: start_scan_line,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });
    antic_data.insert_mode_line(&ModeLineDescr {
        mode: 2,
        scan_line: start_scan_line + 8,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });
    let voffs = antic_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 50, 101, 97, 100, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });
    scr_offsets.0[1] = voffs;

    antic_data.insert_mode_line(&ModeLineDescr {
        mode: 2,
        scan_line: start_scan_line + 16,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });
    let voffs = antic_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });
    scr_offsets.0[2] = voffs;

    antic_data.insert_mode_line(&ModeLineDescr {
        mode: 2,
        scan_line: start_scan_line + 24,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });

    antic_data.insert_mode_line(&ModeLineDescr {
        mode: 2,
        scan_line: start_scan_line + 32,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs0,
        charset_memory_offset: coffs,
    });
    antic_data.insert_mode_line(&ModeLineDescr {
        mode: 0,
        scan_line: start_scan_line + 40,
        width: 256,
        height: 8,
        n_bytes: 0,
        line_voffset: 0,
        data_offset: 0,
        chbase: 0,
        pmbase: 0,
        hscrol: 0,
        video_memory_offset: voffs,
        charset_memory_offset: coffs,
    });

    let voffs = antic_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 50, 101, 97, 100, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });
    scr_offsets.0[3] = voffs;

    let voffs = antic_data.reserve_antic_memory(40, &mut |data| {
        data.copy_from_slice(&[
            0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    });
    scr_offsets.0[4] = voffs;
    for scan_line in 0..240 {
        antic_data.set_gtia_regs(
            scan_line,
            &GTIARegs {
                hposp: [64 - 4, 96 - 4, 128 - 4, 160 - 4],
                hposm: [192 - 4, 194 - 4, 196 - 4, 198 - 4],
                sizep: [16, 16, 16, 16],
                sizem: 0,
                grafp: [0x55, 0x55, 0x55, 0x55],
                grafm: 0x55,
                col: [0x2a, 0x4a, 0x6a, 0x8a, 40, 202, 148, 70, 0],
                prior: 4,
                vdelay: 0,
                gractl: 0,
                hitclr: 0,
                consol: 0,
            },
        )
    }

    let antic_data_handle = antic_data_assets.add(antic_data);

    commands.spawn().insert_bundle((antic_data_handle,));

    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite::default(),
        texture: main_image_handle,
        transform: Transform {
            scale: Vec3::new(2.0, 2.0, 2.0),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn_bundle(Camera2dBundle::default());
}

const COLLISIONS: bool = cfg!(feature="webgl");

// #[bevy_main]
fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::rgb(0.3, 0.0, 0.6)));
    app.insert_resource(WindowDescriptor {
        width: 384.0 * 2.0,
        height: 240.0 * 2.0,
        scale_factor_override: Some(1.0),
        ..Default::default()
    });
    app.insert_resource(Msaa { samples: 1 });
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
        let log_filter = if let Ok(Some(log_filter)) = local_storage.get_item("log") {
            log_filter
        } else {
            "info".to_string()
        };
        app.insert_resource(bevy::log::LogSettings {
            filter: log_filter,
            level: bevy::utils::tracing::Level::DEBUG,
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.insert_resource(bevy::log::LogSettings {
            level: bevy::utils::tracing::Level::INFO,
            filter: "".to_string(),
        });
    }

    app.add_plugins(DefaultPlugins);
    // .add_plugin(FrameTimeDiagnosticsPlugin::default())
    app.insert_resource(MemOffsets([0; 24]))
        .add_plugin(AtariAnticPlugin {
            collisions: COLLISIONS,
        })
        .add_startup_system(setup)
        .add_system(update);

    // #[cfg(not(target_arch = "wasm32"))]
    // app.add_system(quit_after_few_frames);
    app.run();
}
