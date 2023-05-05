use crate::rdev::{Button, EventType, SimulateError};
use core_graphics::event::{
    CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, ScrollEventUnit,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::convert::TryInto;

use crate::macos::keycodes::code_from_key;

use objc::runtime::Class;

fn pressed_buttons() -> usize {
    let ns_event = Class::get("NSEvent").unwrap();
    unsafe { msg_send![ns_event, pressedMouseButtons] }
}

unsafe fn convert_native_with_source(
    event_type: &EventType,
    source: CGEventSource,
) -> Option<CGEvent> {
    match event_type {
        EventType::KeyPress(key) => {
            let code = code_from_key(*key)?;
            CGEvent::new_keyboard_event(source, code, true).ok()
        }
        EventType::KeyRelease(key) => {
            let code = code_from_key(*key)?;
            CGEvent::new_keyboard_event(source, code, false).ok()
        }
        EventType::ButtonPress(button) => {
            let point = get_current_mouse_location()?;
            let event = match button {
                Button::Left => CGEventType::LeftMouseDown,
                Button::Right => CGEventType::RightMouseDown,
                _ => return None,
            };
            CGEvent::new_mouse_event(
                source,
                event,
                point,
                CGMouseButton::Left, // ignored because we don't use OtherMouse EventType
            )
            .ok()
        }
        EventType::ButtonRelease(button) => {
            let point = get_current_mouse_location()?;
            let event = match button {
                Button::Left => CGEventType::LeftMouseUp,
                Button::Right => CGEventType::RightMouseUp,
                _ => return None,
            };
            CGEvent::new_mouse_event(
                source,
                event,
                point,
                CGMouseButton::Left, // ignored because we don't use OtherMouse EventType
            )
            .ok()
        }
        EventType::MouseMove { x, y } => {
            let pressed = pressed_buttons();

            let event_type = if pressed & 1 > 0 {
                CGEventType::LeftMouseDragged
            } else if pressed & 2 > 0 {
                CGEventType::RightMouseDragged
            } else {
                CGEventType::MouseMoved
            };

            let point = CGPoint { x: (*x), y: (*y) };
            CGEvent::new_mouse_event(source, event_type, point, CGMouseButton::Left).ok()
        }
        /* EventType::MouseMoveRelative { x, y } => {
            let point = get_current_mouse_location()?;
            let new_x = point.x + x;
            let new_y = point.y + y;

            let pressed = pressed_buttons();

            let event_type = if pressed & 1 > 0 {
                CGEventType::LeftMouseDragged
            } else if pressed & 2 > 0 {
                CGEventType::RightMouseDragged
            } else {
                CGEventType::MouseMoved
            };

            let point = CGPoint { x: new_x, y: new_y };
            CGEvent::new_mouse_event(source, event_type, point, CGMouseButton::Left).ok()
        } */
        EventType::Wheel { delta_x, delta_y } => {
            let wheel_count = 2;
            CGEvent::new_scroll_event(
                source,
                ScrollEventUnit::PIXEL,
                wheel_count,
                (*delta_y).try_into().ok()?,
                (*delta_x).try_into().ok()?,
                0,
            )
            .ok()
        }
    }
}

unsafe fn convert_native(event_type: &EventType) -> Option<CGEvent> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    convert_native_with_source(event_type, source)
}

unsafe fn get_current_mouse_location() -> Option<CGPoint> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    let event = CGEvent::new(source).ok()?;
    Some(event.location())
}

// This allows sharing the same source
/* unsafe fn get_current_mouse_location_from_source(source: &CGEventSource) -> Option<CGPoint> {
    let event = CGEvent::new(source.clone()).ok()?;
    Some(event.location())
} */

#[link(name = "Cocoa", kind = "framework")]
extern "C" {}

pub fn simulate(event_type: &EventType) -> Result<(), SimulateError> {
    unsafe {
        if let Some(cg_event) = convert_native(event_type) {
            cg_event.post(CGEventTapLocation::HID);
            Ok(())
        } else {
            Err(SimulateError)
        }
    }
}

unsafe fn convert_native2(x: i32, y: i32) -> (Option<CGEvent>, i32, i32) {
    let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok() {
        Some(source) => source,
        None => return (None, 0, 0),
    };
    _mouse_move_relative(source, x, y)
}

unsafe fn _mouse_move_relative(
    source: CGEventSource,
    x: i32,
    y: i32,
) -> (Option<CGEvent>, i32, i32) {
    let start_point = match get_current_mouse_location() {
        Some(point) => point,
        None => return (None, 0, 0),
    };
    let new_x = start_point.x + (x as f64);
    let new_y = start_point.y + (y as f64);

    let pressed = pressed_buttons();

    let event_type = if pressed & 1 > 0 {
        CGEventType::LeftMouseDragged
    } else if pressed & 2 > 0 {
        CGEventType::RightMouseDragged
    } else {
        CGEventType::MouseMoved
    };

    let point = CGPoint { x: new_x, y: new_y };
    (
        CGEvent::new_mouse_event(source, event_type, point, CGMouseButton::Left).ok(),
        start_point.x as i32,
        start_point.y as i32,
    )
}

pub fn mouse_move_relative(x: i32, y: i32, _return_start_position: bool) -> (i32, i32) {
    unsafe {
        let (cg_event_opt, start_x, start_y) = convert_native2(x, y);
        if let Some(cg_event) = cg_event_opt {
            cg_event.post(CGEventTapLocation::HID);
        }
        (start_x, start_y)
    }
}
