extern crate nalgebra as na;

use skulpin::skia_safe;

use legion::prelude::*;

use skulpin::AppHandler;
use skulpin::CoordinateSystemHelper;
use skulpin::AppControl;
use skulpin::InputState;
use skulpin::TimeState;
use skulpin::VirtualKeyCode;

// Used for physics
use na::Vector2;
use ncollide2d::shape::{Cuboid, ShapeHandle, Ball};
use nphysics2d::object::{ColliderDesc, RigidBodyDesc, Ground, BodyPartHandle, DefaultBodyHandle};

mod physics;
use physics::Physics;

const GROUND_THICKNESS: f32 = 0.2;
const GROUND_HALF_EXTENTS_WIDTH: f32 = 3.0;
const BALL_RADIUS: f32 = 0.2;
const GRAVITY: f32 = -9.81;
const BALL_COUNT: usize = 5;

#[derive(Clone, Copy, Debug)]
struct PaintDesc {
    color: na::Vector4<f32>,
    stroke_width: f32,
}

#[derive(Debug)]
struct DrawSkiaBoxComponent {
    half_extents: na::Vector2<f32>,
    paint: PaintDesc,
}

#[derive(Debug)]
struct DrawSkiaCircleComponent {
    radius: f32,
    paint: PaintDesc,
}

#[derive(Debug)]
struct Position2DComponent {
    position: na::Vector2<f32>,
}

struct RigidBodyComponent {
    handle: DefaultBodyHandle,
}

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

pub struct DemoApp {
    last_fps_text_change: Option<std::time::Instant>,
    fps_text: String,
    physics: Physics,
    #[allow(dead_code)]
    universe: Universe,
    world: World,
}

impl DemoApp {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut physics = Physics::new(Vector2::y() * GRAVITY);

        let universe = Universe::new();
        let mut world = universe.create_world();

        spawn_ground(&mut physics, &mut world);
        spawn_balls(&mut physics, &mut world);

        DemoApp {
            last_fps_text_change: None,
            fps_text: "".to_string(),
            physics,
            universe,
            world,
        }
    }
}

impl AppHandler for DemoApp {
    fn update(
        &mut self,
        app_control: &mut AppControl,
        input_state: &InputState,
        time_state: &TimeState,
    ) {
        let now = time_state.current_instant();

        //
        // Quit if user hits escape
        //
        if input_state.is_key_down(VirtualKeyCode::Escape) {
            app_control.enqueue_terminate_process();
        }

        //
        // Update FPS once a second
        //
        let update_text_string = match self.last_fps_text_change {
            Some(last_update_instant) => (now - last_update_instant).as_secs_f32() >= 1.0,
            None => true,
        };

        // Refresh FPS text
        if update_text_string {
            let fps = time_state.updates_per_second();
            self.fps_text = format!("Fps: {:.1}", fps);
            self.last_fps_text_change = Some(now);
        }

        // Update physics
        self.physics.step();

        // Copy the position of all rigid bodies into their position component
        let query = <(Write<Position2DComponent>, Read<RigidBodyComponent>)>::query();
        for (mut pos, body) in query.iter(&mut self.world) {
            pos.position = self
                .physics
                .bodies
                .rigid_body(body.handle)
                .unwrap()
                .position()
                .translation
                .vector;
        }
    }

    fn draw(
        &mut self,
        _app_control: &AppControl,
        _input_state: &InputState,
        _time_state: &TimeState,
        canvas: &mut skia_safe::Canvas,
        coordinate_system_helper: &CoordinateSystemHelper,
    ) {
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
        let query = <(Read<Position2DComponent>, Read<DrawSkiaBoxComponent>)>::query();
        for (pos, skia_box) in query.iter(&mut self.world) {
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
        let query = <(Read<Position2DComponent>, Read<DrawSkiaCircleComponent>)>::query();
        for (pos, skia_circle) in query.iter(&mut self.world) {
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
        let mut text_paint =
            skia_safe::Paint::new(skia_safe::Color4f::new(1.0, 1.0, 0.0, 1.0), None);
        text_paint.set_anti_alias(true);
        text_paint.set_style(skia_safe::paint::Style::StrokeAndFill);
        text_paint.set_stroke_width(1.0);

        let mut font = skia_safe::Font::default();
        font.set_size(20.0);
        canvas.draw_str(self.fps_text.clone(), (50, 50), &font, &text_paint);
    }

    fn fatal_error(
        &mut self,
        error: &skulpin::AppError,
    ) {
        println!("{}", error);
    }
}
