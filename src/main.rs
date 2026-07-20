#![deny(warnings)]

mod application;
mod canvas;
mod io;
mod model;
mod panels;
mod preferences;
mod show;
mod window;

fn main() -> glib::ExitCode {
    application::run()
}
