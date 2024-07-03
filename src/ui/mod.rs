pub mod messages;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{cell::RefCell, rc::Rc, time::Duration};

const SPINNER_FRAMES: &[&str] = &[
    "⠁", "⠂", "⠄", "⡀", "⡈", "⡐", "⡠", "⣀", "⣁", "⣂", "⣄", "⣌", "⣔", "⣤", "⣥", "⣦", "⣮", "⣶", "⣷",
    "⣿", "⡿", "⠿", "⢟", "⠟", "⡛", "⠛", "⠫", "⢋", "⠋", "⠍", "⡉", "⠉", "⠑", "⠡", "⢁",
]; // &["◜", "◠", "◝", "◞", "◡", "◟"];

const SPINNER_FRAME_DURATION: Duration = Duration::from_millis(80);

/// An interface to the terminal, for spinners. This is a wrapper around `indicatif::MultiProgress`, and also is
/// `Clone`-able (as it uses Rc internally).
#[derive(Clone, Debug)]
pub struct Interface {
    /// The multi-progress bar.
    spinners: Rc<RefCell<MultiProgress>>,

    /// The largest spinner message length.
    max_msg_len: usize,
}

impl Interface {
    /// Creates a new interface.
    pub fn new(max_msg_len: usize) -> Interface {
        Interface {
            spinners: Rc::new(RefCell::new(MultiProgress::new())),
            max_msg_len,
        }
    }

    /// Spawns a new spinner. Returns a handle to the spinner, which can be used to update the spinner.
    pub fn spawn_spinner<S>(&mut self, message: S) -> Spinner
    where
        S: ToString,
    {
        let message = message.to_string();
        let num_dots = self.max_msg_len.saturating_sub(message.len());

        let pb = ProgressBar::new_spinner().with_message(message).with_style(
            ProgressStyle::default_spinner()
                .template(&get_template("{spinner:.blue}", num_dots))
                .expect("This should not fail!")
                .tick_strings(SPINNER_FRAMES),
        );

        let mut spinner = Spinner::new(self.spinners.borrow().add(pb), num_dots);

        spinner.start();

        spinner
    }
}

/// A wrapper around a progress bar.
#[derive(Clone, Debug)]
pub struct Spinner {
    /// The underlying progress bar.
    spinner: Rc<RefCell<ProgressBar>>,

    /// The number of dots to display after the message.
    num_dots: usize,
}

impl Spinner {
    pub fn new(spinner: ProgressBar, num_dots: usize) -> Spinner {
        Spinner {
            spinner: Rc::new(RefCell::new(spinner)),
            num_dots,
        }
    }

    /// Starts the spinner. Note that the spinner does not appear until the first tick.
    pub fn start(&mut self) {
        self.spinner
            .borrow_mut()
            .enable_steady_tick(SPINNER_FRAME_DURATION);
    }

    /// Closes the spinner.
    pub fn close(self) {
        let raw_spinner = self.spinner.borrow();

        raw_spinner.set_style(
            ProgressStyle::default_spinner()
                .template(&get_template("✅", self.num_dots))
                .expect("This should not fail!")
                .tick_strings(SPINNER_FRAMES),
        );

        raw_spinner.finish();
    }
}

fn get_template(ending: &str, num_dots: usize) -> String {
    format!(
        "{{msg}} {dots} {ending}",
        dots = console::style("·".repeat(num_dots)).dim(),
    )
}
