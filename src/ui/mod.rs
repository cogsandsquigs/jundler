use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{cell::RefCell, rc::Rc, time::Duration};

const SPINNER_FRAMES: &[&str] = &[
    "⠁", "⠂", "⠄", "⡀", "⡈", "⡐", "⡠", "⣀", "⣁", "⣂", "⣄", "⣌", "⣔", "⣤", "⣥", "⣦", "⣮", "⣶", "⣷",
    "⣿", "⡿", "⠿", "⢟", "⠟", "⡛", "⠛", "⠫", "⢋", "⠋", "⠍", "⡉", "⠉", "⠑", "⠡", "⢁",
]; // &["◜", "◠", "◝", "◞", "◡", "◟"];

const SPINNER_FRAME_DURATION: Duration = Duration::from_millis(80);

#[derive(Clone, Debug)]
pub struct Interface {
    /// The multi-progress bar.
    spinners: Vec<Spinner>,

    /// The largest spinner message length.
    max_msg_len: usize,
}

impl Interface {
    /// Creates a new interface.
    pub fn new() -> Interface {
        Interface {
            spinners: vec![],
            max_msg_len: 0,
        }
    }

    /// Spawns a new spinner. Returns a handle to the spinner, which can be used to update the spinner.
    pub fn spawn_spinner(&mut self, message: String) -> Spinner {
        // the prev. max message length.
        let prev_max_msg_len = self.max_msg_len;

        // Update the max message length.
        self.max_msg_len = self.max_msg_len.max(message.len());

        // ONLY if the new message length is greater than the previous max message length.
        if prev_max_msg_len != self.max_msg_len {
            // Iterate over all previous spinners, adjusting their lengths according to the new max message length.
            for spinner in self.spinners.iter_mut() {
                let curr_msg_len = spinner.spinner.borrow().message().len();

                let style = ProgressStyle::default_spinner()
                    .template(&get_template(
                        "{spinner:.blue}",
                        self.max_msg_len - curr_msg_len + 3,
                    ))
                    .expect("This should not fail!")
                    .tick_strings(SPINNER_FRAMES);

                spinner.set_style(style);

                spinner.spinner.borrow_mut().tick();
            }
        }

        let spinner = Spinner::new(
            ProgressBar::new_spinner().with_message(message).with_style(
                ProgressStyle::default_spinner()
                    .template(&get_template("{spinner:.blue}", 3))
                    .expect("This should not fail!")
                    .tick_strings(SPINNER_FRAMES),
            ),
        );

        self.spinners.push(spinner.clone());

        spinner
    }
}

/// A wrapper around a progress bar.
#[derive(Clone, Debug)]
pub struct Spinner {
    spinner: Rc<RefCell<ProgressBar>>,
    // pre_msg_pad: String,
    // post_msg_elipses: usize,
    // current: usize,
    // total: usize,
}

impl Spinner {
    //     /// Creates a new spinner.
    //     pub fn new(
    //         message: String,
    //         pre_msg_pad: &str,
    //         post_msg_elipses: usize,
    //         current: usize,
    //         total: usize,
    //     ) -> Spinner {
    //         Spinner {
    //             spinner: ProgressBar::new_spinner()
    //                 .with_message(message)
    //                 // Default spinner style
    //                 .with_style(
    //                     ProgressStyle::default_spinner()
    //                         .template(&template_with_ending(
    //                             "{spinner:.cyan/blue}",
    //                             pre_msg_pad,
    //                             post_msg_elipses,
    //                             current,
    //                             total,
    //                         ))
    //                         .expect("This should not fail!")
    //                         .tick_strings(SPINNER_FRAMES),
    //                 ),
    //             pre_msg_pad: pre_msg_pad.to_string(),
    //             post_msg_elipses,
    //             current,
    //             total,
    //         }
    //     }

    pub fn new(spinner: ProgressBar) -> Spinner {
        Spinner {
            spinner: Rc::new(RefCell::new(spinner)),
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
                .template(&get_template("✅", 3))
                .expect("This should not fail!")
                .tick_strings(SPINNER_FRAMES),
        );

        raw_spinner.finish();
    }

    fn set_style(&mut self, style: ProgressStyle) {
        self.spinner.borrow_mut().set_style(style);
    }

    //     /// Closes the spinner.
    //     pub fn close(self) {
    //         self.spinner.set_style(
    //             ProgressStyle::default_spinner()
    //                 .template(&template_with_ending(
    //                     "✅",
    //                     &self.pre_msg_pad,
    //                     self.post_msg_elipses,
    //                     self.current,
    //                     self.total,
    //                 ))
    //                 .expect("This should not fail!")
    //                 .tick_strings(SPINNER_FRAMES),
    //         );

    //         self.spinner.tick();

    //         self.spinner.finish();
    //     }
}

// /// Creates a template with an attached spinner count and padding, with a custom ending.
// fn template_with_ending(
//     ending: &str,
//     pre_msg_pad: &str,
//     post_msg_elipses: usize,
//     current: usize,
//     total: usize,
// ) -> String {
//     format!(
//         "{}{} {{msg}} ...{} {}",
//         pre_msg_pad,
//         console::style(format!("[{}/{}]", current, total))
//             .bold()
//             .dim(),
//         ".".repeat(post_msg_elipses),
//         ending,
//     )
// }

fn get_template(ending: &str, num_dots: usize) -> String {
    format!(
        "{{msg}} {dots} {ending}",
        dots = console::style("·".repeat(num_dots)).dim(),
    )
}
