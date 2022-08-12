use crate::qt_pkt::QTPacket;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use std::io::Error;

pub struct AudioStreamDescription {
    sample_rate: f64,
    format_id: u32,
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    bytes_per_frame: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
    reserved: u32,
}

pub const AUDIO_FORMAT_ID_LPCM: u32 = 0x6C70636D;

impl AudioStreamDescription {
    pub fn new(
        sample_rate: f64,
        format_id: u32,
        format_flags: u32,
        bytes_per_packet: u32,
        frames_per_packet: u32,
        bytes_per_frame: u32,
        channels_per_frame: u32,
        bits_per_channel: u32,
    ) -> AudioStreamDescription {
        AudioStreamDescription {
            sample_rate,
            format_flags,
            format_id,
            bytes_per_packet,
            frames_per_packet,
            bytes_per_frame,
            channels_per_frame,
            bits_per_channel,
            reserved: 0,
        }
    }

    pub fn from_qt_packet(pkt: &mut QTPacket) -> Result<AudioStreamDescription, Error> {
        let sample_rate = match pkt.read_f64() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let format_id = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let format_flags = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let bytes_per_packet = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let frames_per_packet = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let bytes_per_frame = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let channels_per_frame = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let bits_per_channel = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let reserved = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        Ok(AudioStreamDescription {
            sample_rate,
            format_id,
            format_flags,
            bytes_per_packet,
            frames_per_packet,
            bytes_per_frame,
            channels_per_frame,
            bits_per_channel,
            reserved,
        })
    }

    pub fn default() -> AudioStreamDescription {
        AudioStreamDescription {
            sample_rate: 48000f64,
            format_flags: 12,
            format_id: AUDIO_FORMAT_ID_LPCM,
            bytes_per_packet: 1,
            frames_per_packet: 1,
            bytes_per_frame: 4,
            channels_per_frame: 2,
            bits_per_channel: 16,
            reserved: 0,
        }
    }

    pub fn as_buffer(&self) -> Result<Vec<u8>, Error> {
        let mut buffer: Vec<u8> = Vec::new();

        match buffer.write_f64::<LittleEndian>(self.sample_rate) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_u32::<LittleEndian>(self.format_id) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_u32::<LittleEndian>(self.format_flags) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_u32::<LittleEndian>(self.bytes_per_packet) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_u32::<LittleEndian>(self.frames_per_packet) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_u32::<LittleEndian>(self.bytes_per_frame) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_u32::<LittleEndian>(self.channels_per_frame) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_u32::<LittleEndian>(self.bits_per_channel) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_u32::<LittleEndian>(self.reserved) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_f64::<LittleEndian>(self.sample_rate) {
            Err(e) => return Err(e),
            _ => {}
        };
        match buffer.write_f64::<LittleEndian>(self.sample_rate) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(buffer)
    }
}
