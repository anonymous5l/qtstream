use rusb::{
    Context, Device, DeviceDescriptor, DeviceHandle, Direction, Error, Recipient, RequestType,
    TransferType, UsbContext,
};
use std::thread::sleep;
use std::time::Duration;

pub struct AppleDevice {
    device: Device<Context>,
    descriptor: DeviceDescriptor,
    index_config: u8,
    index_interface: u8,
    index_setting: u8,
    in_max_packet_size: u16,
    out_max_packet_size: u16,
    in_endpoint_address: u8,
    out_endpoint_address: u8,
    handle: DeviceHandle<Context>,
}

impl AppleDevice {
    pub fn new(
        device: Device<Context>,
        descriptor: DeviceDescriptor,
        handle: DeviceHandle<Context>,
    ) -> Self {
        return AppleDevice {
            device,
            descriptor,
            index_config: 0,
            index_interface: 0,
            index_setting: 0,
            in_max_packet_size: 0,
            out_max_packet_size: 0,
            in_endpoint_address: 0,
            out_endpoint_address: 0,
            handle,
        };
    }

    pub fn is_qt_enabled(&self) -> Result<bool, Error> {
        let num_configuration = self.descriptor.num_configurations();
        for config_idx in 0..num_configuration {
            let desc = match self.device.config_descriptor(config_idx) {
                Ok(e) => e,
                Err(e) => return Err(e),
            };

            for interface in desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    if interface_desc.class_code() == 0xFF
                        && interface_desc.sub_class_code() == 0x2A
                    {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    pub fn claim_interface(&mut self) -> Option<Error> {
        let num_configuration = self.descriptor.num_configurations();
        for config_idx in 0..num_configuration {
            let desc = match self.device.config_descriptor(config_idx) {
                Ok(e) => e,
                Err(e) => return Some(e),
            };

            for interface in desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    if interface_desc.class_code() == 0xFF
                        && interface_desc.sub_class_code() == 0x2A
                    {
                        self.index_config = desc.number();
                        self.index_interface = interface_desc.interface_number();
                        self.index_setting = interface_desc.setting_number();

                        if match self.handle.active_configuration() {
                            Err(e) => return Some(e),
                            Ok(cfg) => cfg,
                        } != self.index_config
                        {
                            match self.handle.set_active_configuration(self.index_config) {
                                Err(e) => return Some(e),
                                _ => {}
                            };
                        }

                        match self.handle.claim_interface(self.index_interface) {
                            Err(e) => return Some(e),
                            _ => {}
                        };
                        return None;
                    }
                }
            }
        }
        Some(Error::NotFound)
    }

    pub fn init_bulk_endpoint(&mut self) -> Option<Error> {
        let num_configuration = self.descriptor.num_configurations();
        for config_idx in 0..num_configuration {
            let desc = match self.device.config_descriptor(config_idx) {
                Ok(e) => e,
                Err(e) => return Some(e),
            };

            for interface in desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    if interface_desc.class_code() == 0xFF
                        && interface_desc.sub_class_code() == 0x2A
                    {
                        for endpoint_desc in interface_desc.endpoint_descriptors() {
                            if endpoint_desc.direction() == Direction::In
                                && endpoint_desc.transfer_type() == TransferType::Bulk
                            {
                                self.in_max_packet_size = endpoint_desc.max_packet_size();
                                self.in_endpoint_address = endpoint_desc.address();
                            } else if endpoint_desc.direction() == Direction::Out
                                && endpoint_desc.transfer_type() == TransferType::Bulk
                            {
                                self.out_max_packet_size = endpoint_desc.max_packet_size();
                                self.out_endpoint_address = endpoint_desc.address();
                            }
                        }

                        return None;
                    }
                }
            }
        }
        Some(Error::NotFound)
    }

    pub fn set_qt_enabled(&mut self, enabled: bool) -> Result<bool, Error> {
        let is_enabled = match self.is_qt_enabled() {
            Ok(is_enabled) => is_enabled == enabled,
            Err(e) => return Err(e),
        };

        if is_enabled {
            return Ok(true);
        }

        let index = match enabled {
            true => 2,
            false => 0,
        };

        let buffer: [u8; 0] = [];

        match self.handle.write_control(
            rusb::request_type(Direction::Out, RequestType::Vendor, Recipient::Device),
            0x52,
            0x00,
            index,
            &buffer,
            Duration::from_secs(5),
        ) {
            Err(e) => return Err(e),
            _ => {}
        };

        if enabled {
            sleep(Duration::from_secs(1));

            let context = match Context::new() {
                Ok(ctx) => ctx,
                Err(e) => return Err(e),
            };

            loop {
                self.handle = match context.open_device_with_vid_pid(
                    self.descriptor.vendor_id(),
                    self.descriptor.product_id(),
                ) {
                    Some(e) => e,
                    None => return Err(Error::NotFound),
                };

                self.device = self.handle.device();
                self.descriptor = match self.device.device_descriptor() {
                    Ok(d) => d,
                    Err(e) => return Err(e),
                };

                if match self.is_qt_enabled() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                } == enabled
                {
                    break;
                }

                sleep(Duration::from_millis(500));
            }
        }

        return Ok(true);
    }

    pub fn clear_feature(&self) -> Option<Error> {
        let buffer: [u8; 0] = [];

        match self.handle.write_control(
            rusb::request_type(Direction::Out, RequestType::Standard, Recipient::Endpoint),
            0x01,
            0x00,
            self.in_endpoint_address as u16,
            &buffer,
            Duration::from_secs(1),
        ) {
            Err(e) => return Some(e),
            _ => {}
        };

        match self.handle.write_control(
            rusb::request_type(Direction::Out, RequestType::Standard, Recipient::Endpoint),
            0x01,
            0x00,
            self.out_endpoint_address as u16,
            &buffer,
            Duration::from_secs(1),
        ) {
            Err(e) => return Some(e),
            _ => {}
        };

        None
    }

    pub fn max_read_packet_size(&self) -> u16 {
        self.in_max_packet_size
    }

    pub fn max_write_packet_size(&self) -> u16 {
        self.out_max_packet_size
    }

    pub fn read_bulk(&self, buf: &mut [u8]) -> Result<usize, Error> {
        return self
            .handle
            .read_bulk(self.in_endpoint_address, buf, Duration::from_secs(10));
    }

    pub fn write_bulk(&self, buf: &[u8]) -> Result<usize, Error> {
        return self
            .handle
            .write_bulk(self.out_endpoint_address, buf, Duration::from_secs(10));
    }
}

pub fn get_usb_device(sn: &str) -> Result<AppleDevice, Error> {
    let usb_context = match Context::new() {
        Ok(usb_context) => usb_context,
        Err(e) => return Err(e),
    };

    let devices = match usb_context.devices() {
        Ok(d) => d,
        Err(e) => return Err(e),
    };

    let duration = Duration::from_secs(1);

    for device in devices.iter() {
        let handle = match device.open() {
            Ok(d) => d,
            Err(e) => return Err(e),
        };

        let descriptor = match device.device_descriptor() {
            Ok(d) => d,
            Err(e) => return Err(e),
        };

        let languages = match handle.read_languages(duration) {
            Ok(l) => l,
            Err(e) => return Err(e),
        };

        let usn = match handle.read_serial_number_string(languages[0], &descriptor, duration) {
            Ok(sn) => sn,
            Err(e) => return Err(e),
        };

        let sn_bytes = sn.as_bytes();
        let usn_bytes = &usn.as_bytes()[..sn_bytes.len()];

        if sn_bytes == usn_bytes {
            return Ok(AppleDevice::new(device, descriptor, handle));
        }
    }

    Err(Error::NotFound)
}
