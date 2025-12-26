use std::{
    error::Error,
    fs::File,
    path::PathBuf,
    process::{Command, exit},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
    },
    thread::{self},
    time::Duration,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum MacroItem {
    Mouse(mki::Mouse, i32, i32),
    Key(mki::Keyboard),
}

/// Gets the cordinates of the next mouse click
/// by launching `slurp` (which overlays the screen)
fn get_next_mouseclick_cords() -> Result<(i32, i32), Box<dyn Error>> {
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

    let (x, y): (i32, i32) = (x.parse()?, y.parse()?);

    Ok((x / 2, y / 2))
}

fn unbind_all() {
    mki::remove_any_button_bind();
    mki::remove_any_key_bind();
    mki::remove_key_bind(mki::Keyboard::F1);
}

fn record_macro() -> Result<Vec<MacroItem>, Box<dyn Error>> {
    let macro_vec_mutex = Arc::new(Mutex::new(Vec::<MacroItem>::new()));

    let ignore_esc = Arc::new(AtomicBool::new(false));
    let ignore_esc_ref = ignore_esc.clone();

    let (completion_sender, complation_receiver) = channel();

    let macro_vec_ref1 = macro_vec_mutex.clone();
    mki::bind_any_button(mki::Action::sequencing_mouse(
        move |button| match get_next_mouseclick_cords() {
            Ok((x, y)) => {
                macro_vec_ref1
                    .lock()
                    .unwrap()
                    .push(MacroItem::Mouse(button, x, y));
                println!("Mouse button pressed {:?} at {},{}", button, x, y);
                ignore_esc.store(true, Ordering::SeqCst);
            }
            Err(e) => eprintln!("Ignoring mouse click: {}", e),
        },
    ));
    let macro_vec_ref2 = macro_vec_mutex.clone();
    mki::bind_any_key(mki::Action::sequencing_kb(move |key| {
        if key == mki::Keyboard::Escape && ignore_esc_ref.load(Ordering::SeqCst) {
            eprintln!("Ignoring Escape key (slurp cancel keybind)");
            ignore_esc_ref.store(false, Ordering::SeqCst);
        } else if key == mki::Keyboard::F1 {
            unbind_all();
            completion_sender.send(0).unwrap();
        } else {
            macro_vec_ref2.lock().unwrap().push(MacroItem::Key(key));
            println!("Keyboard key pressed {:?}", key);
        }
    }));

    complation_receiver.recv()?;
    Ok(macro_vec_mutex.lock().unwrap().to_vec())
}

fn play_macro(macro_vec: Vec<MacroItem>) -> Result<(), Box<dyn Error>> {
    use mouce::MouseActions;

    eprintln!("excecuting macro");

    let mut held_keys = Vec::<mki::Keyboard>::new();
    let mouse = mouce::Mouse::new();
    let mut last_pos = (0, 0);
    for i in macro_vec {
        if let MacroItem::Key(key) = i {
            if key == mki::Keyboard::LeftControl {
                key.press();
                held_keys.push(key);
            } else {
                key.click();
                held_keys.iter().for_each(|key| key.release());
                held_keys.clear();
            }
        } else if let MacroItem::Mouse(button, x, y) = i {
            if (x, y) != last_pos {
                mouse.move_to(x, y)?;
                thread::sleep(Duration::from_millis(300));
                last_pos = (x, y);
            }
            button.click();
        }
    }
    Ok(())
}

fn test() {
    mki::bind_key(
        mki::Keyboard::F1,
        mki::Action::handle_kb(|_| {
            unbind_all();

            let macro_vec = record_macro().unwrap();
            eprintln!("{:#?}", macro_vec);

            play_macro(macro_vec).unwrap();

            test();
        }),
    );
    mki::bind_key(mki::Keyboard::F2, mki::Action::handle_kb(|_| exit(0)));
}

fn main() {
    let mut args = std::env::args();
    if let Some(action) = args.nth(1) {
        let macro_path = PathBuf::from(args.nth(1).unwrap_or("default".to_string()) + ".json");
        let (completion_sender, completion_reciever) = channel();

        match action.as_str() {
            "record" => {
                let file = File::create(&macro_path).unwrap(); // FIXME: use create_new to prevent overwriting of important macros

                mki::bind_key(
                    mki::Keyboard::F1,
                    mki::Action::handle_kb(move |_| {
                        serde_json::to_writer(&file, &record_macro().unwrap()).unwrap();
                        completion_sender.send(0).unwrap();
                    }),
                );
            }
            "play" => {
                let file = File::open(&macro_path).unwrap();

                mki::bind_key(
                    mki::Keyboard::F1,
                    mki::Action::handle_kb(move |_| {
                        play_macro(serde_json::from_reader(&file).unwrap()).unwrap();
                        completion_sender.send(0).unwrap();
                    }),
                );
            }
            _ => {
                eprintln!("Unimplemented argument; chose one of: 'record', 'play'");
                exit(1);
            }
        }

        println!("Ready; Press F1 to start");
        completion_reciever.recv().unwrap();
    } else {
        test();
        thread::sleep(Duration::MAX);
    }
}
