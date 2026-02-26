// src/input.rs
use smithay::{
    backend::input::{
        AbsolutePositionEvent, Event, InputBackend, InputEvent, KeyState, KeyboardKeyEvent,
        PointerButtonEvent,
    },
    input::{
        keyboard::{FilterResult, Keysym},
        pointer::{ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
};

use crate::state::Clux;

impl Clux {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);

                self.seat.get_keyboard().unwrap().input::<(), _>(
                    self,
                    event.key_code(),
                    event.state(),
                    serial,
                    time,
                    |state, modifiers, handle| {
                        let keysym = handle.modified_sym();

                        if event.state() == KeyState::Pressed {
                            if (modifiers.ctrl && modifiers.alt && keysym == Keysym::BackSpace)
                                || (modifiers.logo && keysym == Keysym::q)
                                || keysym == Keysym::Escape
                            {
                                state.loop_signal.stop();
                            }
                        }

                        FilterResult::Forward
                    },
                );
            }
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.space.outputs().next().unwrap();
                let output_geo = self.space.output_geometry(output).unwrap();
                let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.seat.get_pointer().unwrap();

                let under = self.surface_under(pos);
                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event, .. } => {
                let serial = SERIAL_COUNTER.next_serial();
                let pointer = self.seat.get_pointer().unwrap();
                let keyboard = self.seat.get_keyboard().unwrap();

                if event.state() == smithay::backend::input::ButtonState::Pressed {
                    let pos = pointer.current_location();
                    // Fix E0502: Clone the window to break the immutable borrow on self.space
                    let window = self.space.element_under(pos).map(|(w, _)| w.clone());

                    if let Some(window) = window {
                        self.space.raise_element(&window, true);
                        if let Some(toplevel) = window.toplevel() {
                            keyboard.set_focus(self, Some(toplevel.wl_surface().clone()), serial);
                        }
                    }
                }

                pointer.button(
                    self,
                    &ButtonEvent {
                        button: event.button_code(),
                        state: event.state().try_into().unwrap(),
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            _ => {}
        }
    }
}
