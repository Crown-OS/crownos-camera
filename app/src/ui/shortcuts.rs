use crownui::ext::{Key, Keybind, Modifiers, NamedKey};

pub const SPACE: Keybind = Keybind {
    key: Key::Named(NamedKey::Space),
    modifiers: Modifiers::NONE,
};
