//! The pointer under X11, read by the click watch ([`super::ClickWatch`]): `QueryPointer` on the
//! root window, which needs no grab.

use std::error::Error;

use x11rb::protocol::xproto::KeyButMask;

use super::Point;
use super::clicks::PointerSource;
use crate::x11::Session;

/// The buttons that count as a click: left, middle and right.
const BUTTONS: [KeyButMask; 3] = [
    KeyButMask::BUTTON1,
    KeyButMask::BUTTON2,
    KeyButMask::BUTTON3,
];

/// The pointer of the X server.
pub(super) struct SystemPointer {
    session: Session,
}

impl SystemPointer {
    /// Connects to the X server.
    pub(super) fn connect() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            session: Session::connect()?,
        })
    }
}

impl PointerSource for SystemPointer {
    fn read(&mut self) -> Result<(Point, bool), Box<dyn Error>> {
        self.session.drop_events();
        let reply = self.session.pointer()?;
        let point = Point {
            x: i32::from(reply.root_x),
            y: i32::from(reply.root_y),
        };
        let down = BUTTONS
            .into_iter()
            .any(|button| reply.mask.contains(button));
        Ok((point, down))
    }
}
