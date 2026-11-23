//! Hello, as an xpui screen: a title, two lines, and Back to leave.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use xpui::{App, NavigationScreen, Screen, Text, View, vstack};

// Measured on the VM: heap 304 B at most, stack 1208 B at most (see
// docs/writing-apps.md); these leave room to spare.
dessen::heap!(1024);
dessen::stack!(2304);

struct Hello;

impl Screen for Hello {
    type Message = ();

    fn title(&self) -> Option<&'static str> {
        Some("Hello")
    }

    fn body(&self) -> impl View<()> {
        NavigationScreen::new(
            vstack![16; Text::new("Hello from xpui"), Text::new("Press Back to leave")],
        )
    }

    fn update(&mut self, _message: ()) {}
}

fn app() -> App {
    App::new(Hello)
}

xpui_dessen::main!(app);
