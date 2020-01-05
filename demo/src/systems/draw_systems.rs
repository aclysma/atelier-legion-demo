use legion::prelude::*;

use skulpin::imgui;
use skulpin::skia_safe;

use crate::components::Position2DComponent;
use crate::components::DrawSkiaBoxComponent;
use crate::components::DrawSkiaCircleComponent;

use crate::resources::CanvasDrawResource;
use crate::resources::ImguiResource;
use crate::resources::FpsTextResource;

pub fn draw() -> Box<dyn Schedulable> {
    // Copy the data from physics rigid bodies into position components
    SystemBuilder::new("draw")
        .write_resource::<CanvasDrawResource>()
        .write_resource::<ImguiResource>()
        .read_resource::<FpsTextResource>()
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
                        let x_half_extents = crate::GROUND_HALF_EXTENTS_WIDTH * 1.5;
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
