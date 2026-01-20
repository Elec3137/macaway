use std::{
    fs::File,
    path::PathBuf,
    process::exit,
    sync::mpsc,
    thread::{self},
    time::Duration,
};

mod input;

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
                        if let Ok(macro_vec) = input::record_macro()
                            .inspect_err(|e| eprintln!("failed to record macro: {e}"))
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
                        }) && input::play_macro(macro_vec)
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
        input::test();
        thread::sleep(Duration::MAX);
    }
}
