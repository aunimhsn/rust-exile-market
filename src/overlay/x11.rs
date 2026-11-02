//! The pointer under X11, read by the click watch ([`super::ClickWatch`]): `QueryPointer` on the
//! root window, which needs no grab.

use std::error::Error;

use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{ConnectionExt as _, KeyButMask, Window};
use x11rb::rust_connection::RustConnection;

use super::Point;
use super::clicks::PointerSource;

/// The buttons that count as a click: left, middle and right.
const BUTTONS: [KeyButMask; 3] = [
    KeyButMask::BUTTON1,
    KeyButMask::BUTTON2,
    KeyButMask::BUTTON3,
];

/// The pointer of the X server.
pub(super) struct SystemPointer {
    connection: RustConnection,
    root: Window,
}

impl SystemPointer {
    pub(super) fn connect() -> Result<Self, Box<dyn Error>> {
        let (connection, screen) = x11rb::connect(None)?;
        let root = connection.setup().roots[screen].root;
        Ok(Self { connection, root })
    }
}

impl PointerSource for SystemPointer {
    fn read(&mut self) -> Result<(Point, bool), Box<dyn Error>> {
        crate::x11::drop_events(&self.connection);
        let reply = self.connection.query_pointer(self.root)?.reply()?;
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
