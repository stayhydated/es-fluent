use crate::core::GenerateResult;

/// Messages that drive the TUI application state machine.
#[derive(Debug)]
pub enum Message {
    /// Advance the throbber animation.
    Tick,

    /// User requested to quit the application.
    Quit,

    /// A file changed in a crate's source directory.
    FileChanged {
        /// Name of the crate where a file changed.
        crate_name: String,
    },

    /// Generation started for a crate.
    GenerationStarted {
        /// Name of the crate being generated.
        crate_name: String,
    },

    /// Generation completed (success or failure).
    GenerationComplete {
        /// Result of the generation.
        result: GenerateResult,
    },

    /// An error occurred in the file watcher.
    WatchError {
        /// Error message to display.
        error: String,
    },
}
