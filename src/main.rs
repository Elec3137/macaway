use std::{process::Command, thread, time::Duration};

use mki::Action;

fn main() {
    mki::bind_any_key(Action::sequencing_kb(|key| {
        println!("Keyboard key pressed {:?}", key);
    }));
    mki::bind_any_button(Action::sequencing_mouse(|button| {
        let mut stdout = String::from_utf8(
            Command::new("slurp")
                .arg("-p")
                .envs([
                    ("XDG_RUNTIME_DIR", "/run/user/1001"),
                    ("WAYLAND_DISPLAY", "wayland-0"),
                ])
                .output()
                .expect("slurp command should be usable")
                .stdout,
        )
        .expect("stdout should be valid utf8");

        if let Some(i) = stdout.find(' ') {
            stdout.truncate(i);
        } else {
            eprintln!("Ignoring click due to missing mouse coordinates");
            return;
        }

        println!("Mouse button pressed {:?} at {:?}", button, stdout);
    }));
    thread::sleep(Duration::from_secs(100));
}
