use crate::{config, controller::Controller, utils};
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{class, define_class, msg_send, sel, ClassType, Ivars, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApp, NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSMenu,
    NSMenuItem, NSStatusBar, NSStatusBarButton, NSStatusItem,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSString, NSTimer};
use std::cell::RefCell;
use std::ptr;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[derive(Debug)]
    struct AppDelegate {
        controller: RefCell<Controller>,
        status_item: RefCell<Option<Retained<NSStatusItem>>>,
        menu: RefCell<Option<Retained<NSMenu>>>,
        timer: RefCell<Option<Retained<NSTimer>>>,
    }

    impl AppDelegate {
        #[unsafe(method_id(init))]
        fn init(this: Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(Ivars::<Self> {
                controller: RefCell::new(Controller::new()),
                status_item: RefCell::new(None),
                menu: RefCell::new(None),
                timer: RefCell::new(None),
            });
            unsafe { msg_send![super(this), init] }
        }

        #[unsafe(method(tick:))]
        fn tick(&self, _timer: &NSTimer) {
            utils::disable_local_event_suppression();
            self.controller().borrow_mut().update_state();
        }
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, _notification: &NSNotification) {
            self.controller().borrow_mut().start();
            let mtm = MainThreadMarker::new().expect("must be on the main thread");
            let (status_item, menu) = build_status_item(mtm);
            *self.status_item().borrow_mut() = Some(status_item);
            *self.menu().borrow_mut() = Some(menu);
            let timer = schedule_timer(self);
            *self.timer().borrow_mut() = Some(timer);
        }

        #[unsafe(method(applicationWillTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {
            if let Some(timer) = self.timer().borrow_mut().take() {
                unsafe { msg_send![&*timer, invalidate] };
            }
            self.controller().borrow_mut().stop();
        }
    }
);

fn build_status_item(mtm: MainThreadMarker) -> (Retained<NSStatusItem>, Retained<NSMenu>) {
    let status_bar: Retained<NSStatusBar> = unsafe { msg_send![NSStatusBar::class(), systemStatusBar] };
    let status_item: Retained<NSStatusItem> =
        unsafe { msg_send![&*status_bar, statusItemWithLength: -1.0] };
    let button: Option<Retained<NSStatusBarButton>> = unsafe { msg_send![&*status_item, button] };
    if let Some(button) = button {
        let title = NSString::from_str("Lapsus");
        unsafe { msg_send![&*button, setTitle: &*title] };
    }
    let menu: Retained<NSMenu> = unsafe { msg_send![NSMenu::class(), new] };
    let quit_title = NSString::from_str("Quit Lapsus");
    let quit_key = NSString::from_str("q");
    let quit_item: Retained<NSMenuItem> = unsafe {
        msg_send![
            NSMenuItem::alloc(),
            initWithTitle: &*quit_title,
            action: sel!(terminate:),
            keyEquivalent: &*quit_key
        ]
    };
    let app = NSApp(mtm);
    unsafe { msg_send![&*quit_item, setTarget: &*app] };
    unsafe { msg_send![&*menu, addItem: &*quit_item] };
    unsafe { msg_send![&*status_item, setMenu: &*menu] };
    (status_item, menu)
}

fn schedule_timer(target: &AppDelegate) -> Retained<NSTimer> {
    unsafe {
        msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: config().min_dt,
            target: target,
            selector: sel!(tick:),
            userInfo: ptr::null::<AnyObject>(),
            repeats: true
        ]
    }
}

pub fn run() {
    let mtm = MainThreadMarker::new().expect("must be on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let delegate: Retained<AppDelegate> = unsafe { msg_send![AppDelegate::class(), new] };
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
}
