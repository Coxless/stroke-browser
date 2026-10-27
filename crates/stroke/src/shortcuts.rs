//! Phase 1 の仮のキー。ページに渡す前に外枠が受け取る。
//!
//! 正式なキー配置とモードは Phase 2 で `crates/stroke-keys` に作る。それまでは、
//! ページの文字入力とぶつからないよう、修飾キー付きのキーと F キーだけを使う。

use winit::keyboard::{Key, ModifiersState, NamedKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// URL 入力欄を開く
    OpenLocation,
    Back,
    Forward,
    Reload,
    Quit,
}

/// 押されたキーが外枠の命令なら、その命令を返す。`None` ならページに渡す。
pub fn command_for(key: &Key, modifiers: ModifiersState) -> Option<Command> {
    let ctrl = modifiers == ModifiersState::CONTROL;
    let alt = modifiers == ModifiersState::ALT;
    let none = modifiers.is_empty();
    match key {
        Key::Character(c) if ctrl && c.eq_ignore_ascii_case("l") => Some(Command::OpenLocation),
        Key::Character(c) if alt && c.eq_ignore_ascii_case("d") => Some(Command::OpenLocation),
        Key::Named(NamedKey::F6) if none => Some(Command::OpenLocation),
        Key::Named(NamedKey::ArrowLeft) if alt => Some(Command::Back),
        Key::Named(NamedKey::ArrowRight) if alt => Some(Command::Forward),
        Key::Character(c) if ctrl && c.eq_ignore_ascii_case("r") => Some(Command::Reload),
        Key::Named(NamedKey::F5) if none => Some(Command::Reload),
        Key::Character(c) if ctrl && c.eq_ignore_ascii_case("q") => Some(Command::Quit),
        _ => None,
    }
}

/// 状態表示に出す、キーの一覧
pub const HELP: &str = "Ctrl+L: URL  Alt+←/→: 戻る/進む  F5: 再読み込み  Ctrl+Q: 終了";

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(c: &str) -> Key {
        Key::Character(c.into())
    }

    #[test]
    fn shell_keys() {
        let ctrl = ModifiersState::CONTROL;
        let alt = ModifiersState::ALT;
        let none = ModifiersState::empty();
        assert_eq!(command_for(&ch("l"), ctrl), Some(Command::OpenLocation));
        assert_eq!(command_for(&ch("d"), alt), Some(Command::OpenLocation));
        assert_eq!(
            command_for(&Key::Named(NamedKey::F6), none),
            Some(Command::OpenLocation)
        );
        assert_eq!(
            command_for(&Key::Named(NamedKey::ArrowLeft), alt),
            Some(Command::Back)
        );
        assert_eq!(
            command_for(&Key::Named(NamedKey::ArrowRight), alt),
            Some(Command::Forward)
        );
        assert_eq!(command_for(&ch("r"), ctrl), Some(Command::Reload));
        assert_eq!(
            command_for(&Key::Named(NamedKey::F5), none),
            Some(Command::Reload)
        );
        assert_eq!(command_for(&ch("q"), ctrl), Some(Command::Quit));
    }

    #[test]
    fn other_keys_go_to_the_page() {
        let none = ModifiersState::empty();
        assert_eq!(command_for(&ch("l"), none), None);
        assert_eq!(command_for(&ch("L"), ModifiersState::SHIFT), None);
        assert_eq!(
            command_for(&ch("l"), ModifiersState::CONTROL | ModifiersState::SHIFT),
            None
        );
        assert_eq!(command_for(&Key::Named(NamedKey::ArrowLeft), none), None);
        assert_eq!(command_for(&Key::Named(NamedKey::Space), none), None);
    }
}
