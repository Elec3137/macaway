use std::{
    io,
    process::{Child, Command},
    thread,
    time::Duration,
};

pub struct Ydotool {
    daemon_handler: Child,
}

#[allow(dead_code)]
impl Ydotool {
    pub fn start_daemon() -> io::Result<Self> {
        let handler = Command::new("ydotoold").spawn()?;
        Ok(Ydotool {
            daemon_handler: handler,
        })
    }

    fn reset_mouse_pos() -> io::Result<()> {
        Command::new("ydotool")
            .args(["mousemove", "--absolute", "-x", "0", "-y", "0"])
            .spawn()?
            .wait()?;
        Ok(())
    }
    pub fn move_mouse(x: u16, y: u16) -> io::Result<()> {
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

    pub fn click() -> io::Result<()> {
        Command::new("ydotool")
            .args(["click", "C0"])
            .spawn()?
            .wait()?;
        Ok(())
    }
    pub fn click_at(x: u16, y: u16) -> io::Result<()> {
        Ydotool::move_mouse(x, y)?;
        Ydotool::click()?;

        Ok(())
    }
}

impl Drop for Ydotool {
    fn drop(&mut self) {
        #[allow(unused_must_use)]
        self.daemon_handler.kill();
    }
}
