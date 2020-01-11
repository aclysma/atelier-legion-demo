use legion::prelude::*;

use skulpin::imgui;
use skulpin::skia_safe;

use crate::components::Position2DComponent;
use crate::components::DrawSkiaBoxComponent;
use crate::components::DrawSkiaCircleComponent;

use crate::resources::{CanvasDrawResource, CameraResource, InputResource, ViewportResource, DebugDrawResource};
use crate::resources::ImguiResource;
use crate::resources::FpsTextResource;

use skulpin::winit;
use skulpin::LogicalSize;

pub fn draw() -> Box<dyn Schedulable> {
    // Copy the data from physics rigid bodies into position components
    SystemBuilder::new("draw")
        .write_resource::<CanvasDrawResource>()
        .write_resource::<ImguiResource>()
        .read_resource::<FpsTextResource>()
        .write_resource::<CameraResource>()
        .write_resource::<ViewportResource>()
        .read_resource::<InputResource>()
        .write_resource::<DebugDrawResource>()
        .with_query(<(Read<Position2DComponent>, Read<DrawSkiaBoxComponent>)>::query())
        .with_query(<(Read<Position2DComponent>, Read<DrawSkiaCircleComponent>)>::query())
        .build(
            |_,
             world,
             (draw_context, imgui_manager, fps_text, camera_state, viewport_state, input_resource, debug_draw),
             (draw_boxes_query, draw_circles_query)| {
                imgui_manager.with_ui(|ui| {
                    draw_context.with_canvas(|canvas, coordinate_system_helper| {
                        // Set up the coordinate system such that Y position is in the upward direction
                        let x_half_extents = crate::GROUND_HALF_EXTENTS_WIDTH * 1.5;
                        let y_half_extents = x_half_extents
                            / (coordinate_system_helper.surface_extents().width as f32
                                / coordinate_system_helper.surface_extents().height as f32);

                        let window_size = input_resource.window_size();
                        let camera_position = camera_state.position;
                        camera_state.view_half_extents = glm::Vec2::new(x_half_extents, y_half_extents);
                        viewport_state.update(window_size, camera_position, camera_state.view_half_extents);

                        coordinate_system_helper
                            .use_visible_range(
                                canvas,
                                skia_safe::Rect {
                                    left: -x_half_extents + camera_position.x,
                                    right: x_half_extents + camera_position.x,
                                    top: y_half_extents + camera_position.y,
                                    bottom: -y_half_extents + camera_position.y,
                                },
                                skia_safe::matrix::ScaleToFit::Center,
                            )
                            .unwrap();

                        // Generally would want to clear data every time we draw
                        canvas.clear(skia_safe::Color::from_argb(0, 0, 0, 255));

                        // Draw all the boxes
                        for (pos, skia_box) in draw_boxes_query.iter(world) {
                            let paint = skia_box.paint.0.lock().unwrap();
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
                            let paint = skia_circle.paint.0.lock().unwrap();
                            canvas.draw_circle(
                                skia_safe::Point::new(pos.position.x, pos.position.y),
                                skia_circle.radius,
                                &paint,
                            );
                        }

                        // Debug draw
                        for line_list in debug_draw.take_line_lists() {
                            if line_list.points.len() < 2 {
                                continue;
                            }

                            let paint = skia_safe::Paint::new(
                                skia_safe::Color4f::new(
                                    line_list.color.x,
                                    line_list.color.y,
                                    line_list.color.z,
                                    line_list.color.w),
                                None
                            );

                            let from = line_list.points[0];
                            let mut from = skia_safe::Point::new(from.x, from.y);
                            for i in 1..line_list.points.len() {
                                let to = line_list.points[i];
                                let to = skia_safe::Point::new(to.x, to.y);
                                canvas.draw_line(from, to, &paint);
                                from = to;
                            }
                        }

                        debug_draw.clear();


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
