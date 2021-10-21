use bevy::{PipelinedDefaultPlugins, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, ecs::prelude::*, math::Vec3, prelude::{App, Assets, GlobalTransform, Handle, HandleUntyped, Transform}, reflect::TypeUuid, render2::{camera::OrthographicCameraBundle, mesh::Mesh, renderer::RenderDevice}, window::WindowDescriptor};
use bevy_atari_antic::atari_data::{AtariData, AtariDataInner};
use bevy_atari_antic::AtariAnticPlugin;

pub const ANTIC_MESH_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Mesh::TYPE_UUID, 16056864393442354012);

fn main() {
    let mut app = App::new();
    app.insert_resource(WindowDescriptor {
        width: 768.0,
        height: 480.0,
        scale_factor_override: Some(1.0),
        ..Default::default()
    });
    #[cfg(target_arch = "wasm32")]
    {
        let local_storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
        if let Ok(Some(log_filter)) = local_storage.get_item("log") {
            app.insert_resource(bevy::log::LogSettings {
                filter: log_filter,
                level: bevy::utils::tracing::Level::INFO,
            });
        }
    }
    app.add_plugins(PipelinedDefaultPlugins)
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(AtariAnticPlugin)
        .add_startup_system(setup)
        .add_system(update)
        // .add_system(update)
        .init_resource::<AtariData>()
        .run();
}

fn update(
    mut atari_data_assets: ResMut<Assets<AtariData>>,
    query: Query<&Handle<AtariData>>,
) {
    for handle in query.iter() {
        if let Some(atari_data) = atari_data_assets.get_mut(handle) {
            let mut inner = atari_data.inner.write();
            inner.memory[1024] += 1;
            inner.memory[1024 + 31] += 1;
        }

    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    render_device: Res<RenderDevice>,
    mut atari_data_assets: ResMut<Assets<AtariData>>,
) {
    let mut atari_data = AtariData::default();
    atari_data.insert_mode_line(104, 256, 8, 0, 0, 0, 1024, 0);
    atari_data.insert_mode_line(112, 256, 8, 2, 0, 0, 1024, 0);
    atari_data.insert_mode_line(120, 256, 8, 2, 0, 0, 1024 + 40, 0);
    atari_data.insert_mode_line(128, 256, 8, 2, 0, 0, 1024 + 80, 0);
    atari_data.insert_mode_line(136, 256, 8, 2, 0, 0, 1024, 0);
    atari_data.insert_mode_line(144, 256, 8, 0, 0, 0, 1024, 0);

    let mem = atari_data.reserve_antic_memory(include_bytes!("charset.dat"));
    atari_data.reserve_antic_memory(&[
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);

    atari_data.reserve_antic_memory(&[
        0, 0, 50, 101, 97, 100, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);

    atari_data.reserve_antic_memory(&[
        0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);

    let mesh = atari_data.create_mesh();

    {
        let mut atari_data = atari_data.inner.write();

        for scan_line in 0..240 {
            atari_data.gtia1.0[scan_line].colors[0] = 0;
            atari_data.gtia1.0[scan_line].colors[1] = 40;
            atari_data.gtia1.0[scan_line].colors[2] = 202;
            atari_data.gtia1.0[scan_line].colors[3] = 148;
            atari_data.gtia1.0[scan_line].colors[4] = 70;
            atari_data.gtia1.0[scan_line].colors[5] = 0;
            atari_data.gtia1.0[scan_line].colors[6] = 0;
            atari_data.gtia1.0[scan_line].colors[7] = 0;

            atari_data.gtia1.0[scan_line].colors_pm[0] = 0x2a;
            atari_data.gtia1.0[scan_line].colors_pm[1] = 0x4a;
            atari_data.gtia1.0[scan_line].colors_pm[2] = 0x6a;
            atari_data.gtia1.0[scan_line].colors_pm[3] = 0x8a;

            atari_data.gtia3.0[scan_line].hposp[0] = 64.0 - 4.0;
            atari_data.gtia3.0[scan_line].hposp[1] = 96.0 - 4.0;
            atari_data.gtia3.0[scan_line].hposp[2] = 128.0 - 4.0;
            atari_data.gtia3.0[scan_line].hposp[3] = 160.0 - 4.0;
            atari_data.gtia3.0[scan_line].hposm =
                [192.0 - 4.0, 194.0 - 4.0, 196.0 - 4.0, 198.0 - 4.0];
            atari_data.gtia3.0[scan_line].grafm = 0x55;
            atari_data.gtia3.0[scan_line].prior = 4;

            atari_data.gtia2.0[scan_line].player_size[0] = 16.0;
            atari_data.gtia2.0[scan_line].player_size[1] = 16.0;
            atari_data.gtia2.0[scan_line].player_size[2] = 16.0;
            atari_data.gtia2.0[scan_line].player_size[3] = 16.0;
            atari_data.gtia2.0[scan_line].missile_size = [4.0, 4.0, 4.0, 4.0];

            atari_data.gtia2.0[scan_line].grafp[0] = 0x55;
            atari_data.gtia2.0[scan_line].grafp[1] = 0x55;
            atari_data.gtia2.0[scan_line].grafp[2] = 0x55;
            atari_data.gtia2.0[scan_line].grafp[3] = 0x55;
        }
    }

    let atari_data_handle = atari_data_assets.add(atari_data);

    let mesh_handle = meshes.add(mesh);

    // cube
    commands.spawn().insert_bundle((
        Transform::from_xyz(-1.0, 0.0, 0.0),
        GlobalTransform::default(),
        mesh_handle,
        atari_data_handle,
    ));

    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.camera.name = Some("camera_3d".to_string());
    camera_bundle.transform.scale = Vec3::new(0.5, 0.5, 1.0);
    camera_bundle.transform.translation = Vec3::new(0.0, 0.0, 0.0);

    // camera
    commands.spawn_bundle(camera_bundle);
}
