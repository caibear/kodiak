// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: LGPL-3.0-or-later

use crate::broker::Apply;
use std::num::NonZeroU8;
use strum::EnumIter;
use strum_macros::Display;

/// Each variant is a possible key. Not guaranteed to support all keys.
#[derive(Copy, Clone, Eq, PartialEq, Display, EnumIter)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,

    Alt,
    Backspace,
    Ctrl,
    Down,
    Enter,
    Escape,
    Home,
    Left,
    PageDown,
    PageUp,
    MinusUnderscore,
    EqualsPlus,
    Right,
    Shift,
    Space,
    Tab,
    Up,
}

impl Key {
    /// Converts from a Javascript KeyboardEvent.code.
    pub fn try_from_code(code: &str) -> Option<Self> {
        Some(match code {
            "Backspace" => Self::Backspace,
            "Tab" => Self::Tab,
            "Enter" => Self::Enter,
            "ShiftLeft" | "ShiftRight" => Self::Shift,
            "ControlLeft" | "ControlRight" => Self::Ctrl,
            "AltLeft" | "AltRight" => Self::Alt,
            "Escape" => Self::Escape,
            "Space" => Self::Space,
            "PageUp" => Self::PageUp,
            "PageDown" => Self::PageDown,
            "ArrowLeft" => Self::Left,
            "ArrowUp" => Self::Up,
            "ArrowRight" => Self::Right,
            "ArrowDown" => Self::Down,
            "Digit0" => Self::Zero,
            "Digit1" => Self::One,
            "Digit2" => Self::Two,
            "Digit3" => Self::Three,
            "Digit4" => Self::Four,
            "Digit5" => Self::Five,
            "Digit6" => Self::Six,
            "Digit7" => Self::Seven,
            "Digit8" => Self::Eight,
            "Digit9" => Self::Nine,
            "KeyA" => Self::A,
            "KeyB" => Self::B,
            "KeyC" => Self::C,
            "KeyD" => Self::D,
            "KeyE" => Self::E,
            "KeyF" => Self::F,
            "KeyG" => Self::G,
            "KeyH" => Self::H,
            "KeyI" => Self::I,
            "KeyJ" => Self::J,
            "KeyK" => Self::K,
            "KeyL" => Self::L,
            "KeyM" => Self::M,
            "KeyN" => Self::N,
            "KeyO" => Self::O,
            "KeyP" => Self::P,
            "KeyQ" => Self::Q,
            "KeyR" => Self::R,
            "KeyS" => Self::S,
            "KeyT" => Self::T,
            "KeyU" => Self::U,
            "KeyV" => Self::V,
            "KeyW" => Self::W,
            "KeyX" => Self::X,
            "KeyY" => Self::Y,
            "KeyZ" => Self::Z,
            "Equal" => Self::EqualsPlus,
            "Minus" => Self::MinusUnderscore,
            _ => return None,
        })
    }

    pub fn digit(self) -> Option<u8> {
        Some(match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::Five => 5,
            Self::Six => 6,
            Self::Seven => 7,
            Self::Eight => 8,
            Self::Nine => 9,
            _ => return None,
        })
    }

    /// `digit_with_ten` minus 1
    pub fn index(self) -> Option<usize> {
        self.digit_with_ten().map(|n| (n.get() - 1) as usize)
    }

    /// Digit but 0 is mapped to ten.
    pub fn digit_with_ten(self) -> Option<NonZeroU8> {
        self.digit()
            .map(|d| NonZeroU8::new(d).unwrap_or(NonZeroU8::new(10).unwrap()))
    }

    pub fn digits_with_ten() -> impl Iterator<Item = Self> + 'static {
        [
            Self::One,
            Self::Two,
            Self::Three,
            Self::Four,
            Self::Five,
            Self::Six,
            Self::Seven,
            Self::Eight,
            Self::Nine,
            Self::Zero,
        ]
        .into_iter()
    }
}

/// The state of any key.
#[derive(Default, Copy, Clone)]
pub enum KeyState {
    /// Stores when, in game time, the key was pressed.
    Down(f32),
    #[default]
    Up,
}

impl KeyState {
    /// Is the key down.
    pub fn is_down(&self) -> bool {
        matches!(self, Self::Down(_))
    }

    /// Is the key up.
    pub fn is_up(&self) -> bool {
        matches!(self, Self::Up)
    }

    /// If both keys are down, combined key state is down (time is earlier of the two).
    /// If only one key is down, combined key state is down with same time.
    /// If both keys are up, combined key state is up.
    pub fn combined(&self, other: &Self) -> Self {
        Self::Down(match (self, other) {
            (Self::Down(t1), &Self::Down(t2)) => t1.min(t2),
            (&Self::Down(t), Self::Up) => t,
            (Self::Up, &Self::Down(t)) => t,
            (Self::Up, Self::Up) => return Self::Up,
        })
    }
}

/// The entire current state of the keyboard.
pub struct KeyboardState {
    pub(crate) states: [KeyState; std::mem::variant_count::<Key>()],
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            states: [KeyState::Up; std::mem::variant_count::<Key>()],
        }
    }
}

impl Apply<KeyboardEvent> for KeyboardState {
    fn apply(&mut self, event: KeyboardEvent) {
        if self.state(event.key).is_down() != event.down {
            *self.state_mut(event.key) = if event.down {
                KeyState::Down(event.time)
            } else {
                KeyState::Up
            };
        }
    }
}

impl KeyboardState {
    /// Immutable reference to state of one key.
    pub fn state(&self, key: Key) -> &KeyState {
        &self.states[key as usize]
    }

    /// Mutable reference to state of one key.
    pub(crate) fn state_mut(&mut self, key: Key) -> &mut KeyState {
        &mut self.states[key as usize]
    }

    /// See `KeyState::is_down`.
    pub fn is_down(&self, key: Key) -> bool {
        self.state(key).is_down()
    }

    /// See `KeyState::is_up`.
    pub fn is_up(&self, key: Key) -> bool {
        self.state(key).is_up()
    }
}

/// A key up or key down event, containing some additional context.
pub struct KeyboardEvent {
    pub key: Key,
    // Ctrl (Linux, Windows) or Command key (Mac).
    pub ctrl: bool,
    pub down: bool,
    pub shift: bool,
    pub time: f32,
}
