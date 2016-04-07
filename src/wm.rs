// This file contains all sorts of abstractions our window manager needs to use.
// This includes the window manager itself, errors it can throw, as well as
// wrappers for key presses and the like.
extern crate xcb;

use std::process::exit;

use xcb::base as base;
use xcb::xkb as xkb;
use xcb::xproto as xproto;

// an error encountered by the WM
pub enum WmError {
    CouldNotConnect(base::ConnError),
    CouldNotAcquireScreen,
    CouldNotRegisterAtom(String),
    //CouldNotSetupXkb,
    OtherWmRunning,
    ConnectionInterrupted,
    IOError
}

impl WmError {
    // handle an error, ie. print error message and exit
    pub fn handle(self) -> ! {
        match self {
            WmError::CouldNotConnect(e) =>
                println!("Could not connect: {:?}", e),
            WmError::CouldNotAcquireScreen =>
                println!("Could not acquire screen."),
            WmError::CouldNotRegisterAtom(s) =>
                println!("Could not register atom. {}", s),
            //WmError::CouldNotSetupXkb =>
            //    println!("Could not setup XKB"),
            WmError::OtherWmRunning =>
                println!("Another WM is running."),
            WmError::ConnectionInterrupted =>
                println!("Connection interrupted."),
            WmError::IOError =>
                println!("IO error occured.")
        };
        exit(1);
    }
}

// a window manager, wrapping a Connection and a root window
pub struct Wm<'a> {
    con: &'a base::Connection,
    root: xproto::Window,
}

impl<'a> Wm<'a> {
    // wrap a connection to initialize a window manager
    pub fn new(con: &'a base::Connection, screen_num: i32)
        -> Result<Wm<'a>, WmError> {
        let setup = con.get_setup();
        if let Some(screen) = setup.roots().nth(screen_num as usize) {
            Ok(Wm {con: &con, root: screen.root()})
        } else {
            Err(WmError::CouldNotAcquireScreen)
        }
    }

    // register and get back atoms
    pub fn get_atoms(&self, names: Vec<&str>)
        -> Result<Vec<xproto::Atom>, WmError> {
        let mut cookies = Vec::with_capacity(names.len());
        let mut res = Vec::with_capacity(names.len());
        for name in names {
            cookies.push((xproto::intern_atom(self.con, false, name), name));
        }
        for (cookie, name) in cookies {
            match cookie.get_reply() {
                Ok(r) => res.push(r.atom()),
                Err(_) =>
                    return Err(WmError::CouldNotRegisterAtom(name.to_string()))
            }
        }
        Ok(res)
    }

    // register window manager, by requesting substructure redirects for
    // the root window and registering all events we are interested in
    pub fn register(&self) -> Result<(), WmError> {
        let values
            = xproto::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xproto::EVENT_MASK_PROPERTY_CHANGE
            | xproto::EVENT_MASK_KEY_PRESS
            | xproto::EVENT_MASK_BUTTON_PRESS;
        match xproto::change_window_attributes(self.con, self.root,
            &[(xproto::CW_EVENT_MASK, values)]).request_check() {
            Ok(()) => Ok(()),
            Err(_) => Err(WmError::OtherWmRunning)
        }
    }

    // main loop: wait for events, handle them
    pub fn run(&self) -> Result<(), WmError> {
        loop {
            self.con.flush();
            if let Err(_) = self.con.has_error() {
                return Err(WmError::ConnectionInterrupted);
            }
            match self.con.wait_for_event() {
                Some(ev) => self.handle(ev),
                None => return Err(WmError::IOError)
            }
        }
    }

    // handle an event received from the X server
    fn handle(&self, event: base::GenericEvent) {
        match event.response_type() {
            xkb::STATE_NOTIFY => {
                let ev: &xkb::StateNotifyEvent = base::cast_event(&event);
                println!("Key pressed: type:{}, code:{}",
                         ev.xkbType(), ev.keycode());
            },
            xproto::BUTTON_PRESS => {
                let ev: &xproto::ButtonPressEvent = base::cast_event(&event);
                println!("Button pressed: button:{}, x:{}, y:{}",
                         ev.detail(), ev.root_x(), ev.root_y());
            }
            xproto::PROPERTY_NOTIFY => {
                let ev: &xproto::PropertyNotifyEvent =
                    base::cast_event(&event);
                println!("Property changed for window {}: {}",
                         ev.window(), ev.atom());
            }
            xproto::CREATE_NOTIFY => {
                let ev: &xproto::CreateNotifyEvent = base::cast_event(&event);
                println!("Parent {} created window {} at x:{}, y:{}",
                         ev.parent(), ev.window(), ev.x(), ev.y());
            }
            xproto::DESTROY_NOTIFY => {
                let ev: &xproto::DestroyNotifyEvent = base::cast_event(&event);
                println!("Window {} destroyed", ev.window());
            }
            xproto::CONFIGURE_REQUEST => {
                let ev: &xproto::ConfigureRequestEvent
                    = base::cast_event(&event);
                println!("Window {} changes geometry", ev.window());
            }
            xproto::MAP_REQUEST => {
                let ev: &xproto::MapRequestEvent = base::cast_event(&event);
                println!("Window {} requests mapping", ev.window());
            }
            num => println!("Unknown event number: {}.", num)
        }
    }
}