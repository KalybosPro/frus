// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **Permissions on macOS** (milestone 733): [`crate::Permission`] through `AVCaptureDevice` for
//! the camera and the microphone, and `CLLocationManager` for the location.
//!
//! macOS **ends a process** that asks for one of these when its bundle does not say why it wants
//! it, with no error to catch. So nothing is asked without the usage string: the status is
//! [`PermissionStatus::Blocked`] and the log names the key to add, which also covers a program
//! run outside a bundle.
//!
//! The answer is read from the status, the same way for both: it stops being "not determined"
//! when the person has answered.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_foundation::NSString;

use crate::permission::{from_macos_capture, from_macos_location};
use crate::{Permission, PermissionStatus};

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

#[link(name = "CoreLocation", kind = "framework")]
extern "C" {}

extern "C" {
    /// The main queue, which `dispatch_get_main_queue()` is a macro for.
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(
        queue: *const c_void,
        context: *mut c_void,
        work: extern "C" fn(*mut c_void),
    );
}

/// How often the status is looked at while the question is up.
const POLL: Duration = Duration::from_millis(200);

/// How long a question is waited for. Someone who walked away is not answering; the status is
/// still "not determined", and the application may ask again.
const PATIENCE: Duration = Duration::from_secs(300);

/// `AVMediaTypeVideo` and `AVMediaTypeAudio`, whose values are part of the system's interface.
fn media_type(permission: Permission) -> Option<&'static str> {
    match permission {
        Permission::Camera => Some("vide"),
        Permission::Microphone => Some("soun"),
        _ => None,
    }
}

/// The keys of the bundle's `Info.plist` that may say why `permission` is wanted.
fn usage_keys(permission: Permission) -> &'static [&'static str] {
    match permission {
        Permission::Camera => &["NSCameraUsageDescription"],
        Permission::Microphone => &["NSMicrophoneUsageDescription"],
        Permission::Location => &[
            "NSLocationWhenInUseUsageDescription",
            "NSLocationUsageDescription",
        ],
        Permission::Notifications => &[],
    }
}

fn says_why(permission: Permission) -> bool {
    usage_keys(permission).iter().any(|key| {
        let key = NSString::from_str(key);
        // SAFETY: `mainBundle` and `objectForInfoDictionaryKey:` have these signatures.
        let value: Option<Retained<AnyObject>> = unsafe {
            let bundle: Retained<AnyObject> = msg_send![class!(NSBundle), mainBundle];
            msg_send![&*bundle, objectForInfoDictionaryKey: &*key]
        };
        value.is_some()
    })
}

/// The system's answer, with no regard for the usage strings. `None` when its class is not there.
fn system_status(permission: Permission) -> Option<PermissionStatus> {
    if let Some(media) = media_type(permission) {
        let device = AnyClass::get(c"AVCaptureDevice")?;
        let media = NSString::from_str(media);
        // SAFETY: `authorizationStatusForMediaType:` takes a string and returns an NSInteger.
        let status: isize = unsafe { msg_send![device, authorizationStatusForMediaType: &*media] };
        return Some(from_macos_capture(status));
    }
    let manager = AnyClass::get(c"CLLocationManager")?;
    // SAFETY: both are class methods, returning a BOOL and a CLAuthorizationStatus (an int32).
    let (enabled, status): (Bool, i32) = unsafe {
        (
            msg_send![manager, locationServicesEnabled],
            msg_send![manager, authorizationStatus],
        )
    };
    Some(from_macos_location(enabled.as_bool(), status))
}

/// Where `permission` stands. Asking is not offered when asking would end the process.
pub(crate) fn status(permission: Permission) -> PermissionStatus {
    autoreleasepool(|_| {
        // The notification centre asks by itself, the first time one is shown.
        if permission == Permission::Notifications {
            return PermissionStatus::Granted;
        }
        match system_status(permission) {
            Some(PermissionStatus::Ask) if !says_why(permission) => {
                log::warn!(
                    "{permission} is refused without asking: the application's Info.plist does \
                     not say why it wants it ({})",
                    usage_keys(permission).join(" or ")
                );
                PermissionStatus::Blocked
            }
            Some(status) => status,
            None => PermissionStatus::Unavailable,
        }
    })
}

/// Asks the location on the main thread: a location manager reports to the run loop of the
/// thread that made it, and only a thread with one shows the question. The manager is kept for
/// the life of the process, as the question goes away with it.
extern "C" fn ask_location(_: *mut c_void) {
    autoreleasepool(|_| {
        let Some(class) = AnyClass::get(c"CLLocationManager") else {
            return;
        };
        // SAFETY: `new` makes a manager, and `requestWhenInUseAuthorization` takes nothing.
        unsafe {
            let manager: Retained<AnyObject> = msg_send![class, new];
            let _: () = msg_send![&*manager, requestWhenInUseAuthorization];
            std::mem::forget(manager);
        }
    });
}

fn ask(permission: Permission) {
    if let Some(media) = media_type(permission) {
        autoreleasepool(|_| {
            let Some(device) = AnyClass::get(c"AVCaptureDevice") else {
                return;
            };
            let media = NSString::from_str(media);
            let answered = RcBlock::new(|_granted: Bool| {});
            // SAFETY: the method takes a string and a block, which it copies.
            let _: () = unsafe {
                msg_send![
                    device,
                    requestAccessForMediaType: &*media,
                    completionHandler: &*answered
                ]
            };
        });
    } else {
        // SAFETY: the main queue lives as long as the process, and the work takes no context.
        unsafe {
            dispatch_async_f(
                std::ptr::addr_of!(_dispatch_main_q),
                std::ptr::null_mut(),
                ask_location,
            );
        }
    }
}

/// Asks for `permission` when there is something to ask, and waits for the answer — on a thread
/// made for waiting.
pub(crate) async fn request(permission: Permission) -> PermissionStatus {
    blocking::unblock(move || {
        let before = status(permission);
        if before != PermissionStatus::Ask {
            return before;
        }
        ask(permission);
        let started = Instant::now();
        while status(permission) == PermissionStatus::Ask && started.elapsed() < PATIENCE {
            std::thread::sleep(POLL);
        }
        status(permission)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test's program has no bundle, so nothing here may ask: if it did, macOS would end the
    /// process and this would not fail but vanish. Reading the system's answer also runs the
    /// class lookups and the message signatures for real.
    #[test]
    fn with_no_usage_string_nothing_is_asked() {
        for permission in [
            Permission::Camera,
            Permission::Microphone,
            Permission::Location,
        ] {
            assert!(!says_why(permission));
            assert!(
                system_status(permission).is_some(),
                "{permission}: the class is not there"
            );
            let status = futures_lite::future::block_on(request(permission));
            assert_ne!(status, PermissionStatus::Ask, "{permission}");
            assert_ne!(status, PermissionStatus::Unavailable, "{permission}");
        }
    }

    #[test]
    fn notifications_are_left_to_the_notification_centre() {
        assert_eq!(status(Permission::Notifications), PermissionStatus::Granted);
    }
}
