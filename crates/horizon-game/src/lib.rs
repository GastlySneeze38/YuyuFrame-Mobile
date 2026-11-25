//! The game as the platform runs it, for an executable to start (docs/layers.md, "How a Horizon
//! crate plugs in"). The official executable (`horizon-official`, Horizon layer) registers its
//! hooks (`horizon_platform::hooks`) and then calls [`main`]; it never names an engine crate.
//!
//! An executable built on it:
//!
//! ```ignore
//! static GAME_IDENTITY: [u8; horizon_game::GAME_IDENTITY.len()] = horizon_game::GAME_IDENTITY;
//!
//! fn main() {
//!     horizon_game::prepare_main_stack();
//!     std::hint::black_box(&GAME_IDENTITY);
//!     my_crate::register();
//!     horizon_game::main();
//! }
//! ```
//!
//! The platform's own parts register here, in [`main`], as they leave the engine (layer plan M6).
#![forbid(unsafe_code)]

/// Identifies an executable as the game for launcher imports, without an API version or build id.
pub const GAME_IDENTITY: [u8; horizon_mod_package::game_identity::MARK.len()] =
    horizon_mod_package::game_identity::MARK;

/// Runs the game with the hooks registered so far, and returns when it ends. Register hooks
/// (`horizon_platform::hooks::register`) before calling it.
pub fn main() {
    horizon_guest_stack::prepare_main();
    horizon_mod_host::exit_if_compile_child();
    let startup = horizon_client::entry::prepare();
    // The mod lease holder is a process of its own: none on iOS, which starts no other process.
    #[cfg(not(target_os = "ios"))]
    std::hint::black_box(&horizon_mod_package::holder::CAPABILITY);
    horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
}

/// YuyuFrame, iOS: starts the game inside the app that hosts it (the launcher), with `args` for
/// its command line, which an iOS process doesn't have. Call it on the main thread once the
/// app has launched: it returns as soon as the game runs in the app's own loop (winit's
/// `EventLoopExtIOS::spawn_app`), its window in front of the launcher's.
///
/// Once per process: winit's loop and the engine's state are made once, so
/// [`StartError::AlreadyStarted`] (nothing done) when the game already started, a failed start
/// included; `horizon_client::ios::show_game` brings a running game back instead, and another
/// instance needs the app opened again. A start that fails says why
/// ([`StartError::Failed`]) instead of ending the process, which is the host's too.
#[cfg(target_os = "ios")]
pub fn start_in_app(args: Vec<String>) -> Result<(), StartError> {
    static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if STARTED.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return Err(StartError::AlreadyStarted);
    }
    horizon_client::ios::set_args(args);
    main();
    horizon_client::ios::take_start_error().map_or(Ok(()), |e| Err(StartError::Failed(e)))
}

/// Why [`start_in_app`] did not start the game.
#[cfg(target_os = "ios")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartError {
    /// The game started earlier in this process, whatever became of it.
    AlreadyStarted,
    /// The start ended on this error.
    Failed(String),
}

/// Establish the executable main thread stack before initialization.
pub use horizon_guest_stack::prepare_main as prepare_main_stack;
