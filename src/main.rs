use std::{
    error::Error,
    fs::File,
    path::{Path, PathBuf},
    process::{Command, exit},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
    },
    thread::{self},
    time::Duration,
};

use mki::{Action, Keyboard, Mouse};

mod ydotool;
use ydotool::Ydotool;

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

    let (x, y) = stdout
        .split_once(',')
        .ok_or("Failed to find comma in slurp output")?;

    Ok((x.parse()?, y.parse()?))
}

fn unbind_all() {
    mki::remove_any_button_bind();
    mki::remove_any_key_bind();
    mki::remove_key_bind(Keyboard::F1);
}

fn record_macro() -> Result<Vec<(Option<Keyboard>, Option<(Mouse, u16, u16)>)>, Box<dyn Error>> {
    let macro_vec_mutex = Arc::new(Mutex::new(Vec::<(
        Option<Keyboard>,
        Option<(Mouse, u16, u16)>,
    )>::new()));

    let ignore_esc = Arc::new(AtomicBool::new(false));
    let ignore_esc_ref = ignore_esc.clone();

    let (sender, receiver) = channel();

    let macro_vec_ref1 = macro_vec_mutex.clone();
    mki::bind_any_button(Action::sequencing_mouse(
        move |button| match get_next_mouseclick_cords() {
            Ok((x, y)) => {
                macro_vec_ref1
                    .lock()
                    .unwrap()
                    .push((None, Some((button, x, y))));
                println!("Mouse button pressed {:?} at {},{}", button, x, y);
                ignore_esc.store(true, Ordering::SeqCst);
            }
            Err(e) => eprintln!("Ignoring mouse click: {}", e),
        },
    ));
    let macro_vec_ref2 = macro_vec_mutex.clone();
    mki::bind_any_key(Action::sequencing_kb(move |key| {
        if key == Keyboard::Escape && ignore_esc_ref.load(Ordering::SeqCst) {
            eprintln!("Ignoring Escape key (slurp cancel keybind)");
            ignore_esc_ref.store(false, Ordering::SeqCst);
        } else if key == Keyboard::F1 {
            unbind_all();
            sender.send(0).unwrap();
        } else {
            macro_vec_ref2.lock().unwrap().push((Some(key), None));
            println!("Keyboard key pressed {:?}", key);
        }
    }));

    receiver.recv()?;
    Ok(macro_vec_mutex.lock().unwrap().to_vec())
}

fn play_macro(
    macro_vec: Vec<(Option<Keyboard>, Option<(Mouse, u16, u16)>)>,
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

fn test() {
    mki::bind_key(
        Keyboard::F1,
        Action::handle_kb(|_| {
            unbind_all();

            let macro_vec = record_macro().unwrap();
            eprintln!("{:#?}", macro_vec);

            play_macro(macro_vec).unwrap();

            test();
        }),
    );
    mki::bind_key(Keyboard::F2, Action::handle_kb(|_| exit(0)));
}

fn main() {
    let _ydotool: Ydotool;

    let mut args = std::env::args();
    if let Some(action) = args.nth(1) {
        let macro_path = PathBuf::from(args.nth(1).unwrap_or("default".to_string()) + ".json");

        match action.as_str() {
            // FIXME: consider binding the beginning of these actions to a key, ie "F1"
            "record" => {
                let file = File::create(macro_path).unwrap(); // FIXME: use create_new to prevent overwriting of important macros

                serde_json::to_writer(file, &record_macro().unwrap()).unwrap();
            }
            "play" => {
                // FIXME: possible race condition if the daemon doesn't start before the macro starts playing
                _ydotool = Ydotool::start_daemon().unwrap();
                thread::sleep(Duration::from_secs(1));

                let file = File::open(macro_path).unwrap();
                play_macro(serde_json::from_reader(file).unwrap()).unwrap()
            }
            _ => eprintln!("Unimplemented argument; chose one of: 'record', 'play'"),
        }
    } else {
        _ydotool = Ydotool::start_daemon().unwrap();
        test();
        thread::sleep(Duration::MAX);
    }
}
