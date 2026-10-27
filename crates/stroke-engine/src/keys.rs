//! winit のキーイベントを、Servo の受け取る形（W3C の KeyboardEvent）に直す。
//!
//! winit の `NamedKey` と `KeyCode` は、名前が W3C の `key` と `code` の値とほぼ同じなので、
//! 名前の文字列を介して変換する。名前の違うものは個別に直し、それでも合わないものは
//! `Unidentified` にする。servoshell は全部を1つずつ書き並べているが（`keyutils.rs`）、
//! Phase 1 ではこれで足りる。困るキーが出てきたら、ここに足す。

use servo::{Code, Key, KeyState, KeyboardEvent, Location, Modifiers, NamedKey};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{
    Key as WinitKey, KeyCode, KeyLocation, ModifiersState, NamedKey as WinitNamedKey, PhysicalKey,
};

pub(crate) fn keyboard_event(event: &KeyEvent, modifiers: ModifiersState) -> KeyboardEvent {
    let state = match event.state {
        ElementState::Pressed => KeyState::Down,
        ElementState::Released => KeyState::Up,
    };
    let location = match event.location {
        KeyLocation::Standard => Location::Standard,
        KeyLocation::Left => Location::Left,
        KeyLocation::Right => Location::Right,
        KeyLocation::Numpad => Location::Numpad,
    };
    KeyboardEvent::new_without_event(
        state,
        key(&event.logical_key),
        code(event.physical_key),
        location,
        keyboard_modifiers(modifiers),
        event.repeat,
        false,
    )
}

fn key(key: &WinitKey) -> Key {
    match key {
        WinitKey::Character(text) => Key::Character(text.to_string()),
        // W3C では空白は名前のあるキーではなく、文字 " " として扱う
        WinitKey::Named(WinitNamedKey::Space) => Key::Character(" ".into()),
        WinitKey::Named(WinitNamedKey::Super) => Key::Named(NamedKey::Meta),
        WinitKey::Named(named) => Key::Named(
            format!("{named:?}")
                .parse()
                .unwrap_or(NamedKey::Unidentified),
        ),
        WinitKey::Unidentified(_) | WinitKey::Dead(_) => Key::Named(NamedKey::Unidentified),
    }
}

fn code(physical_key: PhysicalKey) -> Code {
    let PhysicalKey::Code(key_code) = physical_key else {
        return Code::Unidentified;
    };
    match key_code {
        KeyCode::SuperLeft => Code::MetaLeft,
        KeyCode::SuperRight => Code::MetaRight,
        other => format!("{other:?}").parse().unwrap_or(Code::Unidentified),
    }
}

fn keyboard_modifiers(state: ModifiersState) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    modifiers.set(Modifiers::CONTROL, state.control_key());
    modifiers.set(Modifiers::SHIFT, state.shift_key());
    modifiers.set(Modifiers::ALT, state.alt_key());
    modifiers.set(Modifiers::META, state.super_key());
    modifiers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_keys_map_by_name() {
        assert_eq!(
            key(&WinitKey::Named(WinitNamedKey::ArrowDown)),
            Key::Named(NamedKey::ArrowDown)
        );
        assert_eq!(
            key(&WinitKey::Named(WinitNamedKey::Enter)),
            Key::Named(NamedKey::Enter)
        );
        assert_eq!(
            key(&WinitKey::Named(WinitNamedKey::PageDown)),
            Key::Named(NamedKey::PageDown)
        );
        assert_eq!(
            key(&WinitKey::Named(WinitNamedKey::F5)),
            Key::Named(NamedKey::F5)
        );
    }

    #[test]
    fn space_is_a_character() {
        assert_eq!(
            key(&WinitKey::Named(WinitNamedKey::Space)),
            Key::Character(" ".into())
        );
    }

    #[test]
    fn codes_map_by_name() {
        assert_eq!(code(PhysicalKey::Code(KeyCode::KeyJ)), Code::KeyJ);
        assert_eq!(code(PhysicalKey::Code(KeyCode::Digit5)), Code::Digit5);
        assert_eq!(code(PhysicalKey::Code(KeyCode::Escape)), Code::Escape);
        assert_eq!(code(PhysicalKey::Code(KeyCode::SuperLeft)), Code::MetaLeft);
    }
}
