use crate::*;

#[test]
fn usb_init_find_busses_devices() {
	unsafe {
		usb_init();
		usb_find_busses();
		usb_find_devices();
	}
}