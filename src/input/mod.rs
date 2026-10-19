//! Input delivery and shell action dispatch.
use smithay::{
    backend::input::{ButtonState, InputBackend, KeyState, KeyboardKeyEvent, PointerButtonEvent},
    input::{
        keyboard::{FilterResult, Keysym, ModifiersState},
        pointer::{ButtonEvent, MotionEvent},
    },
    utils::{Logical, Point, Rectangle},
};

use crate::{
    config,
    desktop::{Interaction, ResizeEdges, State},
    shell::{self, DecorHit, DockItem, LauncherHit, PressKey, QuickAction, TopAction},
    theme::VARIANTS,
};

const BTN_LEFT: u32 = 0x110;

mod keyboard;
pub(crate) use keyboard::*;
mod pointer;
pub(crate) use pointer::*;
mod grab;
use grab::*;
mod dock;
pub(crate) use dock::*;
mod actions;
pub(crate) use actions::*;
mod process;
pub(crate) use process::*;
#[cfg(test)]
mod tests;

mod app_menu;
mod axis;
mod target;
pub(crate) use axis::on_pointer_axis;

mod reset;
pub(crate) use reset::cancel;
mod home;
pub(crate) use home::show_home;
use home::{launch_or_focus, launcher_launch_origin};

mod gestures;
pub(crate) use gestures::{swipe_begin, swipe_end, LauncherSwipe};
