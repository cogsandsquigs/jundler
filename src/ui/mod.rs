use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

const SPINNER_FRAMES: &[&str] = &[
    "⠁", "⠂", "⠄", "⡀", "⡈", "⡐", "⡠", "⣀", "⣁", "⣂", "⣄", "⣌", "⣔", "⣤", "⣥", "⣦", "⣮", "⣶", "⣷",
    "⣿", "⡿", "⠿", "⢟", "⠟", "⡛", "⠛", "⠫", "⢋", "⠋", "⠍", "⡉", "⠉", "⠑", "⠡", "⢁",
]; // &["◜", "◠", "◝", "◞", "◡", "◟"];

const SPINNER_FRAME_DURATION: Duration = Duration::from_millis(80);

#[derive(Clone, Debug)]
pub struct Interface {
    /// The multi-progress bar.
    multi_progress: MultiProgress,
}

impl Interface {
    /// Creates a new interface.
    pub fn new() -> Interface {
        Interface {
            multi_progress: MultiProgress::new(),
        }
    }

    /// Spawns a new spinner. Returns a handle to the spinner, which can be used to update the spinner.
    pub fn spawn_spinner(&mut self, message: String) -> Spinner {
        let spinner = ProgressBar::new_spinner()
            .with_message(message)
            .with_style(ProgressStyle::default_spinner().tick_strings(SPINNER_FRAMES));

        let spinner = self.multi_progress.add(spinner.clone());

        Spinner::new(spinner)
    }
}

/// A wrapper around a progress bar.
#[derive(Clone, Debug)]
pub struct Spinner {
    spinner: ProgressBar,
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
        Spinner { spinner }
    }

    /// Starts the spinner. Note that the spinner does not appear until the first tick.
    pub fn start(&mut self) {
        self.spinner.enable_steady_tick(SPINNER_FRAME_DURATION);
    }

    /// Closes the spinner.
    pub fn close(self) {
        self.spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("This should not fail!")
                .tick_chars("⣿⣿"),
        );
        self.spinner.finish();
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
