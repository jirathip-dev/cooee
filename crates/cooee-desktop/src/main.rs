#![doc = "`cooee-desktop [address]`: the window, for Linux and Windows; macOS builds an empty one, as the Mac has its own app."]
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
fn main() {}

#[cfg(not(target_os = "macos"))]
fn main() -> eframe::Result {
    window::run()
}

#[cfg(not(target_os = "macos"))]
mod window {
    use std::{
        fs,
        path::PathBuf,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use blocks_desktop::Kit;
    use cooee_desktop::Shell;
    use eframe::egui;

    const AGAIN: f64 = 5.0;

    struct App {
        home: PathBuf,
        address: String,
        shell: Option<Shell>,
        tried: f64,
        kit: Kit,
    }

    impl App {
        fn start(&mut self) {
            fs::create_dir_all(&self.home).ok();
            fs::write(self.home.join("address"), &self.address).ok();
            let jar = self.home.join("cookies.txt");
            self.shell = Some(Shell::new(&self.address, jar));
        }
    }

    impl eframe::App for App {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let now = ui.input(|i| i.time);
            egui::CentralPanel::default().show(ui, |ui| {
                let Some(shell) = self.shell.as_mut() else {
                    ui.label("Your cooee server's address, as http://host:port");
                    let typed = ui.text_edit_singleline(&mut self.address);
                    let enter = typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Connect").clicked() || enter {
                        self.start();
                    }
                    return;
                };
                shell.take(Duration::ZERO);
                if !shell.connected() && now - self.tried > AGAIN {
                    self.tried = now;
                    shell.connect();
                }
                let unix = SystemTime::now().duration_since(UNIX_EPOCH);
                shell.show(ui, &self.kit, unix.map_or(0.0, |d| d.as_secs_f64()));
            });
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
    }

    pub(super) fn run() -> eframe::Result {
        let home = std::env::home_dir()
            .unwrap_or_default()
            .join(".cooee-desktop");
        let saved = fs::read_to_string(home.join("address")).unwrap_or_default();
        let address = std::env::args().nth(1).unwrap_or(saved);
        let mut app = App {
            home,
            shell: None,
            tried: 0.0,
            kit: Kit::default(),
            address: address.trim().to_owned(),
        };
        if !app.address.is_empty() {
            app.start();
        }
        let options = eframe::NativeOptions::default();
        eframe::run_native("cooee", options, Box::new(|_| Ok(Box::new(app))))
    }
}
