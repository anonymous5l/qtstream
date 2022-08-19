use crate::qt_pkt::QTPacket;
use byteorder::{LittleEndian, WriteBytesExt};
use std::fmt::{Debug, Formatter};
use std::io::Error;

pub struct Time {
    value: u64,
    scale: u32,
    flags: u32,
    epoch: u64,
}

impl Clone for Time {
    fn clone(&self) -> Self {
        Time {
            value: self.value,
            scale: self.scale,
            flags: self.flags,
            epoch: self.epoch,
        }
    }
}

impl Debug for Time {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "value: {}\nscale: {}\nflags: {}\nepoch: {}\n",
            self.value, self.scale, self.flags, self.epoch,
        ))
    }
}

impl Time {
    pub fn new(value: u64, scale: u32, flags: u32, epoch: u64) -> Time {
        Time {
            value,
            scale,
            flags,
            epoch,
        }
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn get_time_for_scale(&self, new_scale: &Time) -> f64 {
        let scaling_factor = new_scale.scale as f64 / self.scale as f64;
        self.value as f64 * scaling_factor
    }

    pub fn seconds(&self) -> u64 {
        match self.value {
            0 => 0,
            v => v / self.scale as u64,
        }
    }

    pub fn from_qt_packet(pkt: &mut QTPacket) -> Time {
        let value = pkt.read_u64().expect("time read value");
        let scale = pkt.read_u32().expect("time read scale");
        let flags = pkt.read_u32().expect("time read flags");
        let epoch = pkt.read_u64().expect("time read epoch");

        Time {
            value,
            scale,
            flags,
            epoch,
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = Vec::new();

        match buffer.write_u64::<LittleEndian>(self.value) {
            Err(e) => return Err(e),
            _ => {}
        };

        match buffer.write_u32::<LittleEndian>(self.scale) {
            Err(e) => return Err(e),
            _ => {}
        };

        match buffer.write_u32::<LittleEndian>(self.flags) {
            Err(e) => return Err(e),
            _ => {}
        };

        match buffer.write_u64::<LittleEndian>(self.epoch) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(buffer)
    }
}
