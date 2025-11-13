use std::{error::Error, process::Command, thread, time::Duration};

pub struct Ydotool {}

#[allow(dead_code)]
impl Ydotool {
    pub fn start_daemon() -> Result<(), Box<dyn Error>> {
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
    pub fn move_mouse(x: u16, y: u16) -> Result<(), Box<dyn Error>> {
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

    pub fn click() -> Result<(), Box<dyn Error>> {
        Command::new("ydotool")
            .args(["click", "C0"])
            .spawn()?
            .wait()?;
        Ok(())
    }
    pub fn click_at(x: u16, y: u16) -> Result<(), Box<dyn Error>> {
        Ydotool::move_mouse(x, y)?;
        Ydotool::click()?;

        Ok(())
    }
}
