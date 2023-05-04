use crate::linux::common::{FALSE, TRUE};
use crate::linux::keycodes::code_from_key;
use crate::rdev::{Button, EventType, SimulateError};
use std::convert::TryInto;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr::null;
use x11::xlib;
use x11::xtest;

type Window = c_int;

type Xdo = *const c_void;
#[repr(C)]
#[derive(Clone)]
struct WrapperType(Xdo);

unsafe impl Send for WrapperType {}
unsafe impl Sync for WrapperType {}
impl Copy for WrapperType {}

const CURRENT_WINDOW: c_int = 0;
use lazy_static::lazy_static;
use std::ptr;

#[link(name = "xdo")]
extern "C" {
    fn xdo_new(display: *const c_char) -> WrapperType;
    fn xdo_move_mouse_relative(xdo: WrapperType, x: c_int, y: c_int) -> c_int;
    fn xdo_get_mouse_location2(
        xdo: WrapperType,
        x: *mut c_int,
        y: *mut c_int,
        screen: *mut c_int,
        window: *mut Window,
    ) -> c_int;
}

lazy_static! {
    static ref XDO: WrapperType = unsafe { xdo_new(ptr::null()) };
}

fn mouse_location() -> (i32, i32) {
    let mut x = 0;
    let mut y = 0;
    let mut unused_screen_index = 0;
    let mut unused_window_index = CURRENT_WINDOW;
    unsafe {
        xdo_get_mouse_location2(
            *XDO, // TODO save one
            &mut x,
            &mut y,
            &mut unused_screen_index,
            &mut unused_window_index,
        )
    };
    (x, y)
}

unsafe fn send_native(event_type: &EventType, display: *mut xlib::Display) -> Option<()> {
    let res = match event_type {
        EventType::KeyPress(key) => {
            let code = code_from_key(*key)?;
            xtest::XTestFakeKeyEvent(display, code, TRUE, 0)
        }
        EventType::KeyRelease(key) => {
            let code = code_from_key(*key)?;
            xtest::XTestFakeKeyEvent(display, code, FALSE, 0)
        }
        EventType::ButtonPress(button) => match button {
            Button::Left => xtest::XTestFakeButtonEvent(display, 1, TRUE, 0),
            Button::Middle => xtest::XTestFakeButtonEvent(display, 2, TRUE, 0),
            Button::Right => xtest::XTestFakeButtonEvent(display, 3, TRUE, 0),
            Button::Unknown(code) => {
                xtest::XTestFakeButtonEvent(display, (*code).try_into().ok()?, TRUE, 0)
            }
        },
        EventType::ButtonRelease(button) => match button {
            Button::Left => xtest::XTestFakeButtonEvent(display, 1, FALSE, 0),
            Button::Middle => xtest::XTestFakeButtonEvent(display, 2, FALSE, 0),
            Button::Right => xtest::XTestFakeButtonEvent(display, 3, FALSE, 0),
            Button::Unknown(code) => {
                xtest::XTestFakeButtonEvent(display, (*code).try_into().ok()?, FALSE, 0)
            }
        },
        EventType::MouseMove { x, y } => {
            //TODO: replace with clamp if it is stabalized
            let x = if x.is_finite() {
                x.min(c_int::max_value().into())
                    .max(c_int::min_value().into())
                    .round() as c_int
            } else {
                0
            };
            let y = if y.is_finite() {
                y.min(c_int::max_value().into())
                    .max(c_int::min_value().into())
                    .round() as c_int
            } else {
                0
            };
            xtest::XTestFakeMotionEvent(display, 0, x, y, 0)
            //     xlib::XWarpPointer(display, 0, root, 0, 0, 0, 0, *x as i32, *y as i32);
        }
        EventType::Wheel { delta_x, delta_y } => {
            let code_x = if *delta_x > 0 { 7 } else { 6 };
            let code_y = if *delta_y > 0 { 4 } else { 5 };

            let mut result: c_int = 1;
            for _ in 0..delta_x.abs() {
                result = result
                    & xtest::XTestFakeButtonEvent(display, code_x, TRUE, 0)
                    & xtest::XTestFakeButtonEvent(display, code_x, FALSE, 0)
            }
            for _ in 0..delta_y.abs() {
                result = result
                    & xtest::XTestFakeButtonEvent(display, code_y, TRUE, 0)
                    & xtest::XTestFakeButtonEvent(display, code_y, FALSE, 0)
            }
            result
        }
    };
    if res == 0 {
        None
    } else {
        Some(())
    }
}

pub fn simulate(event_type: &EventType) -> Result<(), SimulateError> {
    unsafe {
        let dpy = xlib::XOpenDisplay(null());
        if dpy.is_null() {
            return Err(SimulateError);
        }
        match send_native(event_type, dpy) {
            Some(_) => {
                xlib::XFlush(dpy);
                xlib::XSync(dpy, 0);
                xlib::XCloseDisplay(dpy);
                Ok(())
            }
            None => {
                xlib::XCloseDisplay(dpy);
                Err(SimulateError)
            }
        }
    }
}

pub fn mouse_move_relative(x: i32, y: i32, return_start_position: bool) -> (i32, i32) {
    let mut start_position = (0, 0);
    if return_start_position {
        start_position = mouse_location();
    }
    unsafe {
        xdo_move_mouse_relative(*XDO, x as c_int, y as c_int);
    }
    start_position
}
