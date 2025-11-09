use std::{thread, time::Duration};

use mki::{Action, InhibitEvent, Keyboard, Sequence, bind_key};

fn main() {
    mki::bind_any_button(Action::handle_mouse(|button| {
        println!("Mouse button pressed {:?}", button);
    }));
    thread::sleep(Duration::from_secs(100));
}
