use usb_compat_01_sys::{usb_find_busses, usb_find_devices, usb_get_busses, usb_init};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type BoxResult<T> = Result<T, BoxError>;

fn main() -> BoxResult<()> {
	unsafe {
		usb_init();
		usb_find_busses();
		usb_find_devices();

		let mut bus_ptr = usb_get_busses();
		if !bus_ptr.is_null() {
			loop {
				let mut dev_ptr = (*bus_ptr).devices;
				if !dev_ptr.is_null() {
					loop {
						println!("{:04x}:{:04x}", (*dev_ptr).descriptor.idVendor, (*dev_ptr).descriptor.idProduct);

						if (*dev_ptr).next.is_null() {
							break;
						}
						dev_ptr = (*dev_ptr).next;
					}
				}

				if (*bus_ptr).next.is_null() {
					break;
				}
				bus_ptr = (*bus_ptr).next;
			}
		}
	}

	Ok(())
}