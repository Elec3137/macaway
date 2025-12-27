use std::{
    error::Error,
    fs::File,
    path::PathBuf,
    process::{Command, exit},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
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

fn record_macro() -> Result<Vec<MacroItem>, mpsc::RecvError> {
    let macro_vec_mutex = Arc::new(Mutex::new(Vec::<MacroItem>::new()));
    let (completion_sender, complation_receiver) = mpsc::channel();

    let use_click = Arc::new(AtomicBool::new(true));
    let macro_vec_ref1 = macro_vec_mutex.clone();
    mki::bind_any_button(mki::Action::sequencing_mouse(move |button| {
        if use_click.load(Ordering::SeqCst) {
            match get_next_mouseclick_cords() {
                Ok((x, y)) => {
                    macro_vec_ref1
                        .lock()
                        .unwrap()
                        .push(MacroItem::Mouse(button, x, y));
                    println!("Mouse button pressed {:?} at {},{}", button, x, y);
                    use_click.store(false, Ordering::SeqCst);
                }
                Err(e) => eprintln!("Ignoring mouse click: {}", e),
            }
        } else {
            use_click.store(true, Ordering::SeqCst);
        }
    }));

    let macro_vec_ref2 = macro_vec_mutex.clone();
    mki::bind_any_key(mki::Action::sequencing_kb(move |key| {
        if key == mki::Keyboard::F1 {
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

fn play_macro(macro_vec: Vec<MacroItem>) -> Result<(), mouce::error::Error> {
    use mouce::MouseActions;

    let mut held_keys = Vec::<mki::Keyboard>::new();

    let mouse = mouce::Mouse::new();
    let mut last_pos = (0, 0);

    for item in macro_vec {
        if let MacroItem::Key(key) = item {
            if key == mki::Keyboard::LeftControl
                || key == mki::Keyboard::LeftAlt
                || key == mki::Keyboard::LeftWindows
                || key == mki::Keyboard::RightControl
                || key == mki::Keyboard::RightAlt
                || key == mki::Keyboard::RightWindows
            {
                key.press();
                held_keys.push(key);
            } else {
                key.click();
                held_keys.iter().for_each(|key| key.release());
                held_keys.clear();
            }
        } else if let MacroItem::Mouse(button, x, y) = item {
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
        let macro_path = PathBuf::from(args.nth(0).unwrap_or("default".to_string()) + ".json");
        let (completion_sender, completion_reciever) = mpsc::channel();

        match action.as_str() {
            "record" => {
                let file = File::create(&macro_path).unwrap(); // FIXME: use create_new to prevent overwriting of important macros

                mki::bind_key(
                    mki::Keyboard::F1,
                    mki::Action::handle_kb(move |_| {
                        if let Ok(macro_vec) =
                            record_macro().inspect_err(|e| eprintln!("failed to record macro: {e}"))
                            && serde_json::to_writer(&file, &macro_vec)
                                .inspect_err(|e| {
                                    eprintln!(
                                        "failed to write macro_vec to '{}': {e}",
                                        macro_path.display()
                                    )
                                })
                                .is_ok()
                        {
                            completion_sender.send(true).unwrap();
                        } else {
                            completion_sender.send(false).unwrap();
                        }
                    }),
                );
            }
            "play" => {
                let file = File::open(&macro_path).unwrap();

                mki::bind_key(
                    mki::Keyboard::F1,
                    mki::Action::handle_kb(move |_| {
                        if let Ok(macro_vec) = serde_json::from_reader(&file).inspect_err(|e| {
                            eprintln!(
                                "failed to read macro_vec from '{}': {e}",
                                macro_path.display()
                            )
                        }) && play_macro(macro_vec)
                            .inspect_err(|e| eprintln!("failed to play macro: {e}"))
                            .is_ok()
                        {
                            completion_sender.send(true).unwrap();
                        } else {
                            completion_sender.send(false).unwrap();
                        }
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
