mod fps_text_systems;
pub use fps_text_systems::update_fps_text;

mod physics_systems;
pub use physics_systems::update_physics;
pub use physics_systems::read_from_physics;

mod asset_manager_systems;
pub use asset_manager_systems::update_asset_manager;

mod app_control_systems;
pub use app_control_systems::quit_if_escape_pressed;

mod draw_systems;
pub use draw_systems::draw;

mod time_systems;
pub use time_systems::advance_time;

mod input_systems;
pub use input_systems::input_reset_for_next_frame;

mod editor_systems;
pub use editor_systems::editor_imgui_menu;
pub use editor_systems::editor_keyboard_shortcuts;

use legion::prelude::*;
use legion::schedule::Builder;
use crate::resources::EditorMode;
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ScheduleCriteria {
    is_simulation_paused: bool,
    editor_mode: crate::resources::EditorMode,
}

impl ScheduleCriteria {
    pub fn new(
        is_simulation_paused: bool,
        editor_mode: crate::resources::EditorMode,
    ) -> Self {
        ScheduleCriteria {
            is_simulation_paused,
            editor_mode,
        }
    }
}

//fn always<F>(systems: &mut Vec<Box<dyn Schedulable>>, criteria: &SystemSelectionCriteria, f: F)
//    where F: Fn() -> Box<dyn Schedulable>
//{
//    systems.push((f)())
//}
//
//fn editor_only<F>(systems: &mut Vec<Box<dyn Schedulable>>, criteria: &SystemSelectionCriteria, f: F)
//where F: Fn() -> Box<dyn Schedulable>
//{
//    if criteria.editor_mode == EditorMode::Active {
//        systems.push((f)())
//    }
//}
//
//fn simulation_unpaused_only<F>(systems: &mut Vec<Box<dyn Schedulable>>, criteria: &SystemSelectionCriteria, f: F)
//    where F: Fn() -> Box<dyn Schedulable>
//{
//    if !criteria.is_simulation_paused {
//        systems.push((f)())
//    }
//}

//struct SchedulableListBuilder<'a> {
//    criteria: &'a SystemSelectionCriteria,
//    //schedule: Vec<Box<dyn Schedulable>>
//    schedule: legion::schedule::Builder
//}
//
//impl<'a> SchedulableListBuilder<'a> {
//    fn new(
//        criteria: &'a SystemSelectionCriteria
//    ) {
//        SchedulableListBuilder::<'a> {
//            criteria,
//            schedule: Default::default()
//        }
//    }
//
//    fn build(self) -> Vec<Box<dyn Schedulable>> {
//        self.schedule
//    }
//
//    fn always<F>(mut self, f: F) -> Self
//        where F: Fn() -> Box<dyn Schedulable>
//    {
//        self.schedule.push((f)());
//        self
//    }
//
//    fn editor_only<F>(mut self, f: F) -> Self
//        where F: Fn() -> Box<dyn Schedulable>
//    {
//        if self.criteria.editor_mode == EditorMode::Active {
//            self.schedule.push((f)());
//        }
//
//        self
//    }
//
//    fn simulation_unpaused_only<F>(mut self, f: F) -> Self
//        where F: Fn() -> Box<dyn Schedulable>
//    {
//        if !self.criteria.is_simulation_paused {
//            self.schedule.push((f)());
//        }
//
//        self
//    }
//}

struct ScheduleBuilder<'a> {
    criteria: &'a ScheduleCriteria,
    schedule: legion::schedule::Builder,
}

impl<'a> ScheduleBuilder<'a> {
    fn new(criteria: &'a ScheduleCriteria) -> Self {
        ScheduleBuilder::<'a> {
            criteria,
            schedule: Default::default(),
        }
    }

    fn build(self) -> Schedule {
        self.schedule.build()
    }

    fn always<F>(
        mut self,
        f: F,
    ) -> Self
    where
        F: Fn() -> Box<dyn Schedulable>,
    {
        self.schedule = self.schedule.add_system((f)());
        self
    }

    fn editor_only<F>(
        mut self,
        f: F,
    ) -> Self
    where
        F: Fn() -> Box<dyn Schedulable>,
    {
        if self.criteria.editor_mode == EditorMode::Active {
            self.schedule = self.schedule.add_system((f)());
        }

        self
    }

    fn simulation_unpaused_only<F>(
        mut self,
        f: F,
    ) -> Self
    where
        F: Fn() -> Box<dyn Schedulable>,
    {
        if !self.criteria.is_simulation_paused {
            self.schedule = self.schedule.add_system((f)());
        }

        self
    }
}

pub fn create_update_schedule(criteria: &ScheduleCriteria) -> Schedule {
    //    let mut systems = vec![];
    //    always(&mut systems, criteria, advance_time);
    //    always(&mut systems, criteria, quit_if_escape_pressed);
    //    always(&mut systems, criteria, update_asset_manager);
    //    always(&mut systems, criteria, update_fps_text);
    //    simulation_unpaused_only(&mut systems, criteria, update_physics);
    //    simulation_unpaused_only(&mut systems, criteria, read_from_physics);
    //    // --- Editor stuff here ---
    //    always(&mut systems, criteria, editor_keyboard_shortcuts);
    //    always(&mut systems, criteria, editor_imgui_menu);
    //    // --- End editor stuff ---
    //    always(&mut systems, criteria, input_reset_for_next_frame);

    ScheduleBuilder::new(criteria)
        .always(advance_time)
        .always(quit_if_escape_pressed)
        .always(update_asset_manager)
        .always(update_fps_text)
        .simulation_unpaused_only(update_physics)
        .simulation_unpaused_only(read_from_physics)
        // --- Editor stuff here ---
        .always(editor_keyboard_shortcuts)
        .always(editor_imgui_menu)
        // --- End editor stuff ---
        .always(input_reset_for_next_frame)
        .build()
}

pub fn create_draw_schedule(criteria: &ScheduleCriteria) -> Schedule {
    ScheduleBuilder::new(criteria).always(draw).build()
}
