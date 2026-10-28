//! Global shortcuts as data: which key does what, and when it must stay silent.

use super::View;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Today,
    Show(View),
    NewEvent,
    Step(i32),
}

/// What the key `key` asks for. Nothing while a field or the popover may own the keyboard, with a
/// modifier held (those belong to the system) or, for the one-shot commands, while the key repeats.
pub fn command(key: &str, modified: bool, held: bool, blocked: bool) -> Option<Command> {
    if modified || blocked {
        return None;
    }
    // Caps Lock upper-cases the letter without Shift being held; Shift itself is `modified`.
    match key.to_ascii_lowercase().as_str() {
        "left" => Some(Command::Step(-1)),
        "right" => Some(Command::Step(1)),
        _ if held => None,
        "t" => Some(Command::Today),
        "d" => Some(Command::Show(View::Day)),
        "w" => Some(Command::Show(View::Week)),
        "m" => Some(Command::Show(View::Month)),
        "y" => Some(Command::Show(View::Year)),
        "n" => Some(Command::NewEvent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(key: &str) -> Option<Command> {
        command(key, false, false, false)
    }

    #[test]
    fn each_key_maps_to_its_command() {
        assert_eq!(plain("t"), Some(Command::Today));
        assert_eq!(plain("d"), Some(Command::Show(View::Day)));
        assert_eq!(plain("w"), Some(Command::Show(View::Week)));
        assert_eq!(plain("m"), Some(Command::Show(View::Month)));
        assert_eq!(plain("y"), Some(Command::Show(View::Year)));
        assert_eq!(plain("n"), Some(Command::NewEvent));
        assert_eq!(plain("left"), Some(Command::Step(-1)));
        assert_eq!(plain("right"), Some(Command::Step(1)));
    }

    #[test]
    fn other_keys_do_nothing() {
        for k in ["up", "down", "enter", "escape", "tab", "x", ""] {
            assert_eq!(plain(k), None, "{k}");
        }
    }

    #[test]
    fn caps_lock_does_not_silence_a_letter() {
        // With Caps Lock on, the key can arrive upper-case without Shift held.
        assert_eq!(plain("T"), Some(Command::Today));
        assert_eq!(plain("W"), Some(Command::Show(View::Week)));
    }

    #[test]
    fn blocked_or_modified_keys_are_silent() {
        for k in ["t", "d", "w", "m", "y", "n", "left", "right"] {
            assert_eq!(command(k, false, false, true), None, "{k} while a field has focus");
            assert_eq!(command(k, true, false, false), None, "{k} with a modifier");
        }
    }

    #[test]
    fn a_held_key_repeats_only_the_arrows() {
        assert_eq!(command("right", false, true, false), Some(Command::Step(1)));
        for k in ["t", "d", "w", "m", "y", "n"] {
            assert_eq!(command(k, false, true, false), None, "{k}");
        }
    }
}
