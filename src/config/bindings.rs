//! Compositor-owned keyboard bindings.
/// Which key a chord is compared against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKey {
    /// A printable key, matched on its unmodified character (e.g. `p`).
    Char(char),
    /// The main Return/Enter key.
    Return,
}

/// The action a shell chord performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingAction {
    Home,
    Launcher,
    Terminal,
    ToggleTheme,
    CloseWindow,
}

/// A keyboard chord the shell owns; every other key goes to the focused
/// client. `logo` is the Super/Windows key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    pub logo: bool,
    pub key: BindingKey,
    pub action: BindingAction,
}

/// The compositor's chords, checked in order. Keeping them in one table makes
/// the set testable and the README list easy to audit.
pub const KEY_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        logo: true,
        key: BindingKey::Char('h'),
        action: BindingAction::Home,
    },
    KeyBinding {
        logo: true,
        key: BindingKey::Char('p'),
        action: BindingAction::Launcher,
    },
    KeyBinding {
        logo: true,
        key: BindingKey::Return,
        action: BindingAction::Terminal,
    },
    KeyBinding {
        logo: true,
        key: BindingKey::Char('d'),
        action: BindingAction::ToggleTheme,
    },
    KeyBinding {
        logo: true,
        key: BindingKey::Char('q'),
        action: BindingAction::CloseWindow,
    },
];

/// The action `key` triggers with the given Super state, if the shell owns it.
pub fn binding_for(logo: bool, key: BindingKey) -> Option<BindingAction> {
    KEY_BINDINGS
        .iter()
        .find(|binding| binding.logo == logo && binding.key == key)
        .map(|binding| binding.action)
}
