use std::{
    error::Error,
    process::{Command, exit},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
    },
    thread::{self},
    time::Duration,
};

use mki::{Action, Keyboard};

/// Gets the cordinates of the next mouse click
/// by launching `slurp` (which overlays the screen)
fn get_next_mouseclick_cords() -> Result<(u16, u16), Box<dyn Error>> {
    let mut stdout = String::from_utf8(
        Command::new("slurp")
            .arg("-p") // select a pixel instead of a rectangle
            .envs([
                ("XDG_RUNTIME_DIR", "/run/user/1001"),
                ("WAYLAND_DISPLAY", "wayland-0"),
            ])
            .output()?
            .stdout,
    )?;

    stdout.truncate(
        stdout
            .find(' ')
            .ok_or("Failed to find whitespace in slurp output")?,
    );

    let (x, y) = stdout.split_at(
        stdout
            .find(',')
            .ok_or("Failed to find comma in slurp output")?,
    );

    Ok((x.parse()?, y.replace(",", "").parse()?))
}

struct Ydotool {}

#[allow(dead_code)]
impl Ydotool {
    fn start_daemon() -> Result<(), Box<dyn Error>> {
        Command::new("ydotoold").spawn()?;
        Ok(())
    }

    fn reset_mouse_pos() -> Result<(), Box<dyn Error>> {
        Command::new("ydotool")
            .args(["mousemove", "--absolute", "-x", "0", "-y", "0"])
            .spawn()?
            .wait()?;
        Ok(())
    }
    fn move_mouse(x: u16, y: u16) -> Result<(), Box<dyn Error>> {
        Ydotool::reset_mouse_pos()?;

        thread::sleep(Duration::from_millis(100));

        Command::new("ydotool")
            .args([
                "mousemove",
                "-x",
                &format!("{}", x / 2),
                "-y",
                &format!("{}", y / 2),
            ])
            .spawn()?
            .wait()?;

        Ok(())
    }

    fn click() -> Result<(), Box<dyn Error>> {
        Command::new("ydotool")
            .args(["click", "C0"])
            .spawn()?
            .wait()?;
        Ok(())
    }
    fn click_at(x: u16, y: u16) -> Result<(), Box<dyn Error>> {
        Ydotool::move_mouse(x, y)?;
        Ydotool::click()?;

        Ok(())
    }
}

fn unbind_all() {
    mki::remove_any_button_bind();
    mki::remove_any_key_bind();
    mki::remove_key_bind(mki::Keyboard::F1);
}

fn record_macro()
-> Result<Vec<(Option<mki::Keyboard>, Option<(mki::Mouse, u16, u16)>)>, Box<dyn Error>> {
    let stuff = Arc::new(Mutex::new(Vec::<(
        Option<mki::Keyboard>,
        Option<(mki::Mouse, u16, u16)>,
    )>::new()));
    let stuff_clone = stuff.clone();
    let stuff_clone1 = stuff.clone();

    let ignore_esc = Arc::new(AtomicBool::new(false));
    let ignore_esc_clone = ignore_esc.clone();

    let (sender, receiver) = channel();

    mki::bind_any_button(Action::sequencing_mouse(
        move |button| match get_next_mouseclick_cords() {
            Ok((x, y)) => {
                stuff.lock().unwrap().push((None, Some((button, x, y))));
                println!("Mouse button pressed {:?} at {},{}", button, x, y);
                ignore_esc.store(true, Ordering::SeqCst);
            }
            Err(e) => eprintln!("Ignoring mouse click: {}", e),
        },
    ));
    mki::bind_any_key(Action::sequencing_kb(move |key| {
        if key == mki::Keyboard::Escape && ignore_esc_clone.load(Ordering::SeqCst) {
            eprintln!("Ignoring Escape key (slurp cancel keybind)");
            ignore_esc_clone.store(false, Ordering::SeqCst);
        } else if key == mki::Keyboard::F1 {
            unbind_all();
            sender.send(0).unwrap();
        } else {
            stuff_clone.lock().unwrap().push((Some(key), None));
            println!("Keyboard key pressed {:?}", key);
        }
    }));

    receiver.recv()?;
    Ok(stuff_clone1.lock().unwrap().to_vec())
}

fn play_macro(
    macro_vec: Vec<(Option<mki::Keyboard>, Option<(mki::Mouse, u16, u16)>)>,
) -> Result<(), Box<dyn Error>> {
    eprintln!("excecuting macro");

    let mut held_keys = Vec::<Keyboard>::new();
    for i in macro_vec {
        if let Some(key) = i.0 {
            if key == Keyboard::LeftControl {
                key.press();
                held_keys.push(key);
            } else {
                key.click();
                held_keys.iter().for_each(|key| key.release());
                held_keys.clear();
            }
        } else if let Some(button) = i.1 {
            Ydotool::move_mouse(button.1, button.2)?;
            thread::sleep(Duration::from_millis(100));
            button.0.click();
        }
    }
    Ok(())
}

fn init() {
    mki::bind_key(
        mki::Keyboard::F1,
        Action::handle_kb(|_| {
            unbind_all();

            let macro_vec = record_macro().unwrap();
            eprintln!("{:#?}", macro_vec);

            play_macro(macro_vec).unwrap();

            init();
        }),
    );
    mki::bind_key(mki::Keyboard::F2, Action::handle_kb(|_| exit(0)));
}

fn main() {
    Ydotool::start_daemon().unwrap();
    init();
    thread::sleep(Duration::MAX);
}
