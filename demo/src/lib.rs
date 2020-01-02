extern crate nalgebra as na;

use legion::prelude::*;

use skulpin::skia_safe;

use skulpin::VirtualKeyCode;
use skulpin::imgui;

// Used for physics
use na::Vector2;
use ncollide2d::shape::{Cuboid, ShapeHandle, Ball};
use nphysics2d::object::{ColliderDesc, RigidBodyDesc, Ground, BodyPartHandle};

mod physics;
use physics::Physics;

mod image;

mod asset_manager;
use asset_manager::AssetManager;

mod temp_test;
pub use temp_test::temp_force_load_asset;
pub use temp_test::temp_force_prefab_cook;

mod asset_storage;

mod clone_merge;

pub mod components;

pub mod daemon;

mod prefab_importer;

use components::Position2DComponent;
use components::PaintDesc;
use components::DrawSkiaBoxComponent;
use components::DrawSkiaCircleComponent;
use components::RigidBodyComponent;

mod prefab;
mod prefab_cooking;

pub mod app;

const GROUND_THICKNESS: f32 = 0.2;
const GROUND_HALF_EXTENTS_WIDTH: f32 = 3.0;
const BALL_RADIUS: f32 = 0.2;
const GRAVITY: f32 = -9.81;
const BALL_COUNT: usize = 5;

fn spawn_ground(
    physics: &mut Physics,
    world: &mut World,
) {
    let position = Vector2::y() * -GROUND_THICKNESS;

    // A rectangle that the balls will fall on
    let ground_shape = ShapeHandle::new(Cuboid::new(Vector2::new(
        GROUND_HALF_EXTENTS_WIDTH,
        GROUND_THICKNESS,
    )));

    // Build a static ground body and add it to the body set.
    let ground_body_handle = physics.bodies.insert(Ground::new());

    // Build the collider.
    let ground_collider = ColliderDesc::new(ground_shape)
        .translation(position)
        .build(BodyPartHandle(ground_body_handle, 0));

    // Add the collider to the collider set.
    physics.colliders.insert(ground_collider);

    let paint = PaintDesc {
        color: na::Vector4::new(0.0, 1.0, 0.0, 1.0),
        stroke_width: 0.02,
    };

    world.insert(
        (),
        (0..1).map(|_| {
            (
                Position2DComponent { position },
                DrawSkiaBoxComponent {
                    half_extents: na::Vector2::new(GROUND_HALF_EXTENTS_WIDTH, GROUND_THICKNESS),
                    paint,
                },
            )
        }),
    );
}

fn spawn_balls(
    physics: &mut Physics,
    world: &mut World,
) {
    let ball_shape_handle = ShapeHandle::new(Ball::new(BALL_RADIUS));

    let shift = (BALL_RADIUS + ColliderDesc::<f32>::default_margin()) * 2.0;
    let centerx = shift * (BALL_COUNT as f32) / 2.0;
    let centery = shift / 2.0;
    let height = 3.0;

    let circle_colors = vec![
        na::Vector4::new(0.2, 1.0, 0.2, 1.0),
        na::Vector4::new(1.0, 1.0, 0.2, 1.0),
        na::Vector4::new(1.0, 0.2, 0.2, 1.0),
        na::Vector4::new(0.2, 0.2, 1.0, 1.0),
    ];

    world.insert(
        (),
        (0usize..BALL_COUNT * BALL_COUNT).map(|index| {
            let i = index / BALL_COUNT;
            let j = index % BALL_COUNT;

            let x = i as f32 * shift - centerx;
            let y = j as f32 * shift + centery + height;

            let position = Vector2::new(x, y);

            // Build the rigid body.
            let rigid_body = RigidBodyDesc::new().translation(position).build();

            // Insert the rigid body to the body set.
            let rigid_body_handle = physics.bodies.insert(rigid_body);

            // Build the collider.
            let ball_collider = ColliderDesc::new(ball_shape_handle.clone())
                .density(1.0)
                .build(BodyPartHandle(rigid_body_handle, 0));

            // Insert the collider to the body set.
            physics.colliders.insert(ball_collider);

            (
                Position2DComponent { position },
                DrawSkiaCircleComponent {
                    radius: BALL_RADIUS,
                    paint: PaintDesc {
                        color: circle_colors[index % circle_colors.len()],
                        stroke_width: 0.02,
                    },
                },
                RigidBodyComponent {
                    handle: rigid_body_handle,
                },
            )
        }),
    );
}

/// Create the asset manager that has all the required types registered
pub fn create_asset_manager() -> AssetManager {
    let mut asset_manager = AssetManager::default();
    asset_manager.add_storage::<components::Position2DComponentDefinition>();
    asset_manager.add_storage::<prefab::PrefabAsset>();
    asset_manager
}

struct FpsText {
    pub last_fps_text_change: Option<std::time::Instant>,
    pub fps_text: String,
}

impl FpsText {
    fn new() -> Self {
        FpsText {
            last_fps_text_change: None,
            fps_text: "".to_string(),
        }
    }
}

fn quit_if_escape_pressed() -> Box<dyn Schedulable> {
    SystemBuilder::new("quit_if_escape_pressed")
        .read_resource::<skulpin::InputState>()
        .write_resource::<skulpin::AppControl>()
        .build(|_, _, (input_state, app_control), _| {
            if input_state.is_key_down(VirtualKeyCode::Escape) {
                app_control.enqueue_terminate_process();
            }
        })
}

fn update_asset_manager() -> Box<dyn Schedulable> {
    SystemBuilder::new("update asset manager")
        .write_resource::<AssetManager>()
        .build(|_, _, asset_manager, _| {
            asset_manager.update();
        })
}

fn update_fps_text() -> Box<dyn Schedulable> {
    SystemBuilder::new("update fps text")
        .read_resource::<skulpin::TimeState>()
        .write_resource::<FpsText>()
        .build(|_, _, (time_state, fps_text), _| {
            let now = time_state.current_instant();
            //
            // Update FPS once a second
            //
            let update_text_string = match fps_text.last_fps_text_change {
                Some(last_update_instant) => (now - last_update_instant).as_secs_f32() >= 1.0,
                None => true,
            };

            // Refresh FPS text
            if update_text_string {
                let fps = time_state.updates_per_second();
                fps_text.fps_text = format!("Fps: {:.1}", fps);
                fps_text.last_fps_text_change = Some(now);
            }
        })
}

fn update_physics() -> Box<dyn Schedulable> {
    // Do a physics simulation timestep
    SystemBuilder::new("update physics")
        .write_resource::<Physics>()
        .build(|_, _, physics, _| {
            physics.step();
        })
}

fn read_from_physics() -> Box<dyn Schedulable> {
    SystemBuilder::new("read physics data")
        .read_resource::<Physics>()
        .with_query(<(Write<Position2DComponent>, Read<RigidBodyComponent>)>::query())
        .build(|_, mut world, physics, query| {
            for (mut pos, body) in query.iter(&mut world) {
                pos.position = physics
                    .bodies
                    .rigid_body(body.handle)
                    .unwrap()
                    .position()
                    .translation
                    .vector;
            }
        })
}

fn draw() -> Box<dyn Schedulable> {
    // Copy the data from physics rigid bodies into position components
    SystemBuilder::new("draw")
        .write_resource::<app::DrawContext>()
        .write_resource::<skulpin::ImguiManager>()
        .read_resource::<FpsText>()
        .with_query(<(Read<Position2DComponent>, Read<DrawSkiaBoxComponent>)>::query())
        .with_query(<(Read<Position2DComponent>, Read<DrawSkiaCircleComponent>)>::query())
        .build(
            |_,
             world,
             (draw_context, imgui_manager, fps_text),
             (draw_boxes_query, draw_circles_query)| {
                imgui_manager.with_ui(|ui| {
                    draw_context.with_canvas(|canvas, coordinate_system_helper| {
                        let mut show_demo = true;
                        ui.show_demo_window(&mut show_demo);

                        ui.main_menu_bar(|| {
                            ui.menu(imgui::im_str!("File"), true, || {
                                if imgui::MenuItem::new(imgui::im_str!("New")).build(ui) {
                                    log::info!("clicked");
                                }
                            });
                        });

                        // Set up the coordinate system such that Y position is in the upward direction
                        let x_half_extents = GROUND_HALF_EXTENTS_WIDTH * 1.5;
                        let y_half_extents = x_half_extents
                            / (coordinate_system_helper.surface_extents().width as f32
                                / coordinate_system_helper.surface_extents().height as f32);

                        coordinate_system_helper
                            .use_visible_range(
                                canvas,
                                skia_safe::Rect {
                                    left: -x_half_extents,
                                    right: x_half_extents,
                                    top: y_half_extents + 1.0,
                                    bottom: -y_half_extents + 1.0,
                                },
                                skia_safe::matrix::ScaleToFit::Center,
                            )
                            .unwrap();

                        // Generally would want to clear data every time we draw
                        canvas.clear(skia_safe::Color::from_argb(0, 0, 0, 255));

                        // Draw all the boxes
                        for (pos, skia_box) in draw_boxes_query.iter(world) {
                            let color = skia_safe::Color4f::new(
                                skia_box.paint.color.x,
                                skia_box.paint.color.y,
                                skia_box.paint.color.z,
                                skia_box.paint.color.w,
                            );

                            let mut paint = skia_safe::Paint::new(color, None);
                            paint.set_anti_alias(true);
                            paint.set_style(skia_safe::paint::Style::Stroke);
                            paint.set_stroke_width(skia_box.paint.stroke_width);

                            canvas.draw_rect(
                                skia_safe::Rect {
                                    left: pos.position.x - skia_box.half_extents.x,
                                    right: pos.position.x + skia_box.half_extents.x,
                                    top: pos.position.y - skia_box.half_extents.y,
                                    bottom: pos.position.y + skia_box.half_extents.y,
                                },
                                &paint,
                            );
                        }

                        // Draw all the circles
                        for (pos, skia_circle) in draw_circles_query.iter(world) {
                            let color = skia_safe::Color4f::new(
                                skia_circle.paint.color.x,
                                skia_circle.paint.color.y,
                                skia_circle.paint.color.z,
                                skia_circle.paint.color.w,
                            );

                            let mut paint = skia_safe::Paint::new(color, None);
                            paint.set_anti_alias(true);
                            paint.set_style(skia_safe::paint::Style::Stroke);
                            paint.set_stroke_width(skia_circle.paint.stroke_width);

                            canvas.draw_circle(
                                skia_safe::Point::new(pos.position.x, pos.position.y),
                                skia_circle.radius,
                                &paint,
                            );
                        }

                        // Switch to using logical screen-space coordinates
                        coordinate_system_helper.use_logical_coordinates(canvas);

                        //
                        // Draw FPS text
                        //
                        let mut text_paint = skia_safe::Paint::new(
                            skia_safe::Color4f::new(1.0, 1.0, 0.0, 1.0),
                            None,
                        );
                        text_paint.set_anti_alias(true);
                        text_paint.set_style(skia_safe::paint::Style::StrokeAndFill);
                        text_paint.set_stroke_width(1.0);

                        let mut font = skia_safe::Font::default();
                        font.set_size(20.0);
                        //canvas.draw_str(self.fps_text.clone(), (50, 50), &font, &text_paint);
                        canvas.draw_str(fps_text.fps_text.clone(), (50, 50), &font, &text_paint);
                    });
                });
            },
        )
}

pub struct DemoApp {
    update_schedule: Schedule,
    draw_schedule: Schedule,
}

impl DemoApp {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let update_steps = vec![
            quit_if_escape_pressed(),
            update_asset_manager(),
            update_fps_text(),
            update_physics(),
            read_from_physics(),
        ];

        let mut update_schedule = Schedule::builder();
        for step in update_steps {
            update_schedule = update_schedule.add_system(step);
        }
        let update_schedule = update_schedule.build();

        let draw_schedule = Schedule::builder().add_system(draw()).build();

        DemoApp {
            update_schedule,
            draw_schedule,
        }
    }
}

impl app::AppHandler for DemoApp {
    fn init(
        &mut self,
        world: &mut World,
    ) {
        let asset_manager = create_asset_manager();

        let mut physics = Physics::new(Vector2::y() * GRAVITY);

        spawn_ground(&mut physics, world);
        spawn_balls(&mut physics, world);

        world.resources.insert(physics);
        world.resources.insert(FpsText::new());
        world.resources.insert(asset_manager);
    }

    fn update(
        &mut self,
        world: &mut World,
    ) {
        self.update_schedule.execute(world);
    }

    fn draw(
        &mut self,
        world: &mut World,
    ) {
        // Copy the data from physics rigid bodies into position components
        self.draw_schedule.execute(world);
    }

    fn fatal_error(
        &mut self,
        error: &app::AppError,
    ) {
        println!("{}", error);
    }
}
