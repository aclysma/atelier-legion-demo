use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use imgui_inspect::InspectArgsDefault;
use imgui_inspect::InspectRenderDefault;
use skulpin::imgui;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Vec2 {
    value: glm::Vec2,
}

impl Vec2 {
    pub fn zero() -> Self {
        Vec2 { value: glm::zero() }
    }
}

impl From<glm::Vec2> for Vec2 {
    fn from(value: glm::Vec2) -> Self {
        Vec2 { value }
    }
}

impl Deref for Vec2 {
    type Target = glm::Vec2;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for Vec2 {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl InspectRenderDefault<Vec2> for Vec2 {
    fn render(
        data: &[&Vec2],
        label: &'static str,
        ui: &imgui::Ui,
        _args: &InspectArgsDefault,
    ) {
        if data.len() == 0 {
            return;
        }

        ui.text(&imgui::im_str!("{}: {} {}", label, data[0].x, data[0].y));
    }

    fn render_mut(
        data: &mut [&mut Vec2],
        label: &'static str,
        ui: &imgui::Ui,
        _args: &InspectArgsDefault,
    ) -> bool {
        if data.len() == 0 {
            return false;
        }

        let mut changed = false;
        let mut val = [data[0].x, data[0].y];
        if ui
            .input_float2(&imgui::im_str!("{}", label), &mut val)
            .build()
        {
            changed = true;
            for d in data {
                d.x = val[0];
                d.y = val[1];
            }
        }

        changed
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Vec4 {
    value: glm::Vec4,
}

impl Vec4 {
    pub fn zero() -> Self {
        Vec4 { value: glm::zero() }
    }
}

impl From<glm::Vec4> for Vec4 {
    fn from(value: glm::Vec4) -> Self {
        Vec4 { value }
    }
}

impl Deref for Vec4 {
    type Target = glm::Vec4;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for Vec4 {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl InspectRenderDefault<Vec4> for Vec4 {
    fn render(
        data: &[&Vec4],
        label: &'static str,
        ui: &imgui::Ui,
        _args: &InspectArgsDefault,
    ) {
        if data.len() == 0 {
            return;
        }

        ui.text(&imgui::im_str!(
            "{}: {} {} {} {}",
            label,
            data[0].x,
            data[0].y,
            data[0].z,
            data[0].w
        ));
    }

    fn render_mut(
        data: &mut [&mut Vec4],
        label: &'static str,
        ui: &imgui::Ui,
        _args: &InspectArgsDefault,
    ) -> bool {
        if data.len() == 0 {
            return false;
        }

        let mut changed = false;
        let mut val = [data[0].x, data[0].y, data[0].z, data[0].w];
        if ui
            .input_float4(&imgui::im_str!("{}", label), &mut val)
            .build()
        {
            changed = true;
            for d in data {
                d.x = val[0];
                d.y = val[1];
                d.z = val[2];
                d.w = val[3];
            }
        }

        changed
    }
}
