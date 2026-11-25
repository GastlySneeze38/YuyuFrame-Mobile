//! What the game asks of iOS, as `android.rs` does of Android: the launch arguments, the
//! pasteboard, links and the audio session. Touch, the keyboard, the safe areas and the way back
//! to the launcher are still to come (docs/launcher/ios.md in YuyuFrame).
//!
//! `platform::set` is left alone on purpose: a registered platform means Android's start-up
//! (accounts sealed by its keystore, files prepared by a Java it downloads).
use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{MainThreadMarker, class, msg_send};
use objc2_foundation::{NSDictionary, NSNotification, NSNotificationCenter, NSString, NSURL};
use std::io;
use std::ptr::{NonNull, null_mut};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

#[link(name = "AVFoundation", kind = "framework")]
unsafe extern "C" {
    static AVAudioSessionCategoryAmbient: &'static NSString;
    static AVAudioSessionInterruptionNotification: &'static NSString;
    static AVAudioSessionInterruptionTypeKey: &'static NSString;
}

/// `AVAudioSessionInterruptionTypeBegan`.
const INTERRUPTION_BEGAN: usize = 1;
/// `UIApplicationStateActive`.
const APPLICATION_ACTIVE: isize = 0;

static ARGS: OnceLock<Vec<String>> = OnceLock::new();

/// The game's command line, from the app hosting it: an iOS process gets none. Once, before
/// `entry::prepare`.
pub fn set_args(args: Vec<String>) {
    let _ = ARGS.set(args);
}

pub(crate) fn args() -> Vec<String> {
    ARGS.get().cloned().unwrap_or_default()
}

/// What the game needs of the system before anything else starts.
pub(crate) fn install() {
    horizon_client_input::clipboard::set_platform((clipboard_get, clipboard_set));
    crate::desktop::set_url_opener(open_url);
    open_audio_session();
}

fn pasteboard() -> Option<Retained<AnyObject>> {
    // SAFETY: a class method of UIKit, which the app links, taking nothing.
    unsafe { msg_send![class!(UIPasteboard), generalPasteboard] }
}

fn clipboard_get() -> Option<String> {
    let board = pasteboard()?;
    // SAFETY: `string` is a nullable NSString property of UIPasteboard.
    let text: Option<Retained<NSString>> = unsafe { msg_send![&*board, string] };
    text.map(|text| text.to_string())
}

fn clipboard_set(text: &str) -> bool {
    let Some(board) = pasteboard() else {
        return false;
    };
    let text = NSString::from_str(text);
    // SAFETY: `setString:` takes an NSString, which the pasteboard copies.
    unsafe {
        let () = msg_send![&*board, setString: &*text];
    }
    true
}

fn application() -> Option<Retained<AnyObject>> {
    // SAFETY: a class method of UIKit taking nothing; nil before the application exists.
    unsafe { msg_send![class!(UIApplication), sharedApplication] }
}

/// Opens a link in the system's browser. UIKit takes it on the main thread only.
fn open_url(uri: &str) -> io::Result<()> {
    if MainThreadMarker::new().is_none() {
        return Err(io::Error::other("links open from the main thread"));
    }
    let url = NSURL::URLWithString(&NSString::from_str(uri))
        .ok_or(io::ErrorKind::InvalidInput)?;
    let app = application().ok_or(io::ErrorKind::NotConnected)?;
    let options = NSDictionary::<NSString, AnyObject>::new();
    let done: Option<&DynBlock<dyn Fn(Bool)>> = None;
    // SAFETY: on the main thread, with a URL, an empty options dictionary and no handler, which
    // the method allows.
    unsafe {
        let () = msg_send![
            &*app,
            openURL: &*url,
            options: &*options,
            completionHandler: done
        ];
    }
    Ok(())
}

/// The app's windows (the launcher's and the game's, which winit makes on `Resumed`) on the main
/// thread, with whether each is the game's. None off the main thread or before the app exists.
fn windows() -> Option<Vec<(Retained<AnyObject>, bool)>> {
    MainThreadMarker::new()?;
    let app = application()?;
    // The class of winit's windows (`WinitUIWindow`), registered once the game made one.
    let game_class = AnyClass::get(c"WinitUIWindow");
    // SAFETY: on the main thread: `windows` is an NSArray of UIWindow, read by index below its
    // count, and `isKindOfClass:` takes a class.
    unsafe {
        let list: Retained<AnyObject> = msg_send![&*app, windows];
        let count: usize = msg_send![&*list, count];
        Some(
            (0..count)
                .map(|index| {
                    let window: Retained<AnyObject> = msg_send![&*list, objectAtIndex: index];
                    let game = game_class.is_some_and(|class| {
                        let kind: Bool = msg_send![&*window, isKindOfClass: class];
                        kind.as_bool()
                    });
                    (window, game)
                })
                .collect(),
        )
    }
}

/// Shows the game's windows, or the launcher's, and makes the first of them the key window, the
/// one the touches and the keyboard go to. False when there is none to show.
fn bring_front(game: bool) -> bool {
    let Some(windows) = windows() else {
        return false;
    };
    if !windows.iter().any(|(_, is_game)| *is_game == game) {
        return false;
    }
    let mut keyed = false;
    for (window, is_game) in &windows {
        // SAFETY: on the main thread (`windows` checked it): `setHidden:` and
        // `makeKeyAndVisible` are UIWindow's.
        unsafe {
            if *is_game == game {
                let () = msg_send![&**window, setHidden: Bool::NO];
                if !keyed {
                    let () = msg_send![&**window, makeKeyAndVisible];
                    keyed = true;
                }
            } else {
                let () = msg_send![&**window, setHidden: Bool::YES];
            }
        }
    }
    true
}

/// The pause menu's "Launcher" row: the launcher's window comes in front and the game's hides,
/// the game left open behind it under its pause menu, as on Android. False when the launcher has
/// no window here (the game started on its own), where the row ends the game instead.
pub(crate) fn show_launcher() -> bool {
    bring_front(false)
}

/// "Back to the game", from the launcher: the game's window in front again, where it was left.
/// False when the game has no window (not started yet in this app).
pub fn show_game() -> bool {
    bring_front(true)
}

fn audio_session() -> Option<Retained<AnyObject>> {
    // SAFETY: a class method of AVFoundation, linked above, taking nothing.
    unsafe { msg_send![class!(AVAudioSession), sharedInstance] }
}

/// The session the sound plays in, which cpal leaves to the app. `ambient`: the silent switch
/// mutes the game, as players expect of one, and their own music keeps playing.
fn open_audio_session() {
    let Some(session) = audio_session() else {
        log::warn!("Horizon: no audio session");
        return;
    };
    // SAFETY: a category constant of AVFoundation, and a null error pointer, which it allows.
    let set: Bool = unsafe {
        msg_send![
            &*session,
            setCategory: AVAudioSessionCategoryAmbient,
            error: null_mut::<*mut AnyObject>()
        ]
    };
    if !set.as_bool() {
        log::warn!("Horizon: the audio session refused its category");
    }
    set_session_active(&session);
    watch_interruptions();
}

fn set_session_active(session: &AnyObject) -> bool {
    // SAFETY: `setActive:error:` with a null error pointer, which it allows.
    let active: Bool = unsafe {
        msg_send![session, setActive: Bool::YES, error: null_mut::<*mut AnyObject>()]
    };
    if !active.as_bool() {
        log::warn!("Horizon: the audio session did not start");
    }
    active.as_bool()
}

/// Starts the audio session again, after an interruption or the background. False while the app
/// isn't in front, where iOS refuses it: `resumed` asks again.
pub(crate) fn activate_audio() -> bool {
    if MainThreadMarker::new().is_none() {
        return false;
    }
    let Some(app) = application() else {
        return false;
    };
    // SAFETY: `applicationState` is an NSInteger property, read on the main thread.
    let state: isize = unsafe { msg_send![&*app, applicationState] };
    state == APPLICATION_ACTIVE
        && audio_session().is_some_and(|session| set_session_active(&session))
}

/// A call or an alarm taking the sound, or giving it back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioInterruption {
    Began,
    Ended,
}

/// The last interruption not yet taken: 0 none, 1 began, 2 ended.
static INTERRUPTION: AtomicU8 = AtomicU8::new(0);

/// The last interruption since the last call, for the game's loop.
pub(crate) fn take_audio_interruption() -> Option<AudioInterruption> {
    match INTERRUPTION.swap(0, Ordering::AcqRel) {
        1 => Some(AudioInterruption::Began),
        2 => Some(AudioInterruption::Ended),
        _ => None,
    }
}

/// Keeps the session's interruptions for the game's loop. iOS may post them on any thread, so
/// the observer only stores which one came.
fn watch_interruptions() {
    let block = RcBlock::new(|note: NonNull<NSNotification>| {
        // SAFETY: the notification is live for this call; `userInfo` is a nullable dictionary,
        // `objectForKey:` takes this constant key, and its value is the type's NSNumber.
        let kind: Option<usize> = unsafe {
            let info: Option<Retained<AnyObject>> = msg_send![note.as_ref(), userInfo];
            info.and_then(|info| {
                let value: Option<Retained<AnyObject>> =
                    msg_send![&*info, objectForKey: AVAudioSessionInterruptionTypeKey];
                value.map(|value| msg_send![&*value, unsignedIntegerValue])
            })
        };
        if let Some(kind) = kind {
            let event = if kind == INTERRUPTION_BEGAN { 1 } else { 2 };
            INTERRUPTION.store(event, Ordering::Release);
        }
    });
    let center = NSNotificationCenter::defaultCenter();
    let nothing: Option<&AnyObject> = None;
    // SAFETY: the name is AVFoundation's constant, no object or queue filters it, and the center
    // copies the block. The observer stays for the life of the process, so it is never released.
    unsafe {
        let observer: Retained<AnyObject> = msg_send![
            &*center,
            addObserverForName: AVAudioSessionInterruptionNotification,
            object: nothing,
            queue: nothing,
            usingBlock: &*block
        ];
        std::mem::forget(observer);
    }
}
