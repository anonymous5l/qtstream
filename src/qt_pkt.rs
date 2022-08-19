use crate::coremedia::audio_desc::AudioStreamDescription;
use crate::coremedia::time::Time;
use crate::qt_value::{QTKeyValuePair, QTValue};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::borrow::BorrowMut;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::{BufRead, Cursor, Error, ErrorKind, Read, Seek, SeekFrom, Write};

pub struct QTPacket {
    inner: Cursor<Vec<u8>>,
}

impl QTPacket {
    pub fn new() -> QTPacket {
        let mut cur = Cursor::new(Vec::from([0, 0, 0, 0]));
        cur.seek(SeekFrom::End(0)).expect("cur seek");
        return QTPacket { inner: cur };
    }

    pub fn new_with_magic(magic: u32) -> QTPacket {
        let mut pkt = QTPacket::new();
        pkt.write_u32(magic).expect("write magic");
        pkt
    }

    pub fn read_qt_packet(pkt: &mut QTPacket, size: usize) -> Result<QTPacket, Error> {
        let mut data: Vec<u8> = vec![0; size];
        match pkt.read_exact(&mut data) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let mut new_pkt = QTPacket::new();
        match new_pkt.write(data.as_slice()) {
            Err(e) => return Err(e),
            _ => {}
        };

        // restore position
        match new_pkt.inner.seek(SeekFrom::Start(4)) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(new_pkt)
    }

    pub fn from_qt_packet_with_magic(
        pkt: &mut QTPacket,
        magic: u32,
    ) -> Result<(QTPacket, u32), Error> {
        let mut val_pkt = match QTPacket::from_qt_packet(pkt) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let val_magic = match val_pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        if val_magic != magic {
            return Err(Error::new(ErrorKind::InvalidData, "magic not compare"));
        }

        Ok((val_pkt, val_magic))
    }

    pub fn read_qt_packet_with_magic(&mut self) -> Result<(QTPacket, u32), Error> {
        let mut pkt = match QTPacket::from_qt_packet(self) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let magic = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        Ok((pkt, magic))
    }

    pub fn from_qt_packet(pkt: &mut QTPacket) -> Result<QTPacket, Error> {
        let read_pkt_len = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let pkt_len = match pkt.len() {
            Err(e) => return Err(e),
            Ok(e) => e,
        } as u32;

        if pkt_len < read_pkt_len {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "qt package length not compare data size",
            ));
        }

        let mut buffer: Vec<u8> = vec![0; read_pkt_len as usize];

        if read_pkt_len > 0 {
            match pkt.read_exact(&mut buffer[4..]) {
                Err(e) => return Err(e),
                _ => {}
            };
        }

        let mut cur = Cursor::new(buffer);

        cur.seek(SeekFrom::Start(4)).expect("cur seek");

        Ok(QTPacket { inner: cur })
    }

    pub fn from_bytes(data: &[u8]) -> Result<QTPacket, Error> {
        let pkt_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < pkt_len {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "qt package length not compare data size",
            ));
        }

        let mut cur = Cursor::new(Vec::from(&data[..pkt_len]));

        cur.seek(SeekFrom::Start(4)).expect("cur seek");

        Ok(QTPacket { inner: cur })
    }

    pub fn pos(&mut self) -> u64 {
        return self.inner.position();
    }

    pub fn len(&mut self) -> Result<u64, Error> {
        let cur = self.inner.position();

        let size = match self.inner.seek(SeekFrom::End(0)) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match self.inner.seek(SeekFrom::Start(cur)) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(size)
    }

    pub fn write_u8(&mut self, d: u8) -> Result<(), Error> {
        self.inner.write_u8(d)
    }

    pub fn write_u16(&mut self, d: u16) -> Result<(), Error> {
        self.inner.write_u16::<LittleEndian>(d)
    }

    pub fn write_u32(&mut self, d: u32) -> Result<(), Error> {
        self.inner.write_u32::<LittleEndian>(d)
    }

    pub fn write_f64(&mut self, n: f64) -> Result<(), Error> {
        self.inner.write_f64::<LittleEndian>(n)
    }

    pub fn write_u64(&mut self, d: u64) -> Result<(), Error> {
        self.inner.write_u64::<LittleEndian>(d)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.inner.write(buf)
    }

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        self.inner.read_u8()
    }

    pub fn read_u16(&mut self) -> Result<u16, Error> {
        self.inner.read_u16::<LittleEndian>()
    }

    pub fn read_u32(&mut self) -> Result<u32, Error> {
        self.inner.read_u32::<LittleEndian>()
    }

    pub fn read_f64(&mut self) -> Result<f64, Error> {
        self.inner.read_f64::<LittleEndian>()
    }

    pub fn read_u64(&mut self) -> Result<u64, Error> {
        self.inner.read_u64::<LittleEndian>()
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.inner.read(buf)
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        match self.inner.read_exact(buf) {
            Ok(_size) => Ok(()),
            Err(e) => return Err(e),
        }
    }

    pub fn as_bytes(&mut self) -> Result<&[u8], Error> {
        let pkt_len = self.inner.seek(SeekFrom::End(0)).expect("seek failed") as u32;

        self.inner.seek(SeekFrom::Start(0)).expect("seek failed");
        self.write_u32(pkt_len).expect("write pkg len");
        self.inner.seek(SeekFrom::Start(0)).expect("seek failed");

        self.inner.fill_buf()
    }

    pub fn borrow_mut(&mut self) -> &mut Cursor<Vec<u8>> {
        self.inner.borrow_mut()
    }
}

impl Debug for QTPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "pkt_len: {}\npkt_buf: {}",
                self.inner.get_ref().len(),
                hex::encode(self.inner.get_ref().as_slice())
            )
            .as_str(),
        )
    }
}

pub const PACKET_MAGIC_PING: u32 = 0x70696E67;
pub const PACKET_MAGIC_SYNC: u32 = 0x73796E63;
pub const PACKET_MAGIC_ASYN: u32 = 0x6173796E;

const PACKET_MAGIC_REPLY: u32 = 0x72706C79;

pub struct QTPacketPing {
    header: u64,
}

impl QTPacketPing {
    pub fn new(header: u64) -> QTPacket {
        let mut pkt = QTPacket::new();
        pkt.write_u32(PACKET_MAGIC_PING).unwrap();
        pkt.write_u64(header).unwrap();
        pkt
    }

    pub fn from_packet(pkt: &mut QTPacket) -> Result<QTPacketPing, Error> {
        let header = match pkt.read_u64() {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(QTPacketPing { header })
    }
}

impl Debug for QTPacketPing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("header: {}", self.header).as_str())
    }
}

pub const SYNC_PACKET_MAGIC_OG: u32 = 0x676F2120;
pub const SYNC_PACKET_MAGIC_STOP: u32 = 0x73746F70;
pub const SYNC_PACKET_MAGIC_SKEW: u32 = 0x736B6577;
pub const SYNC_PACKET_MAGIC_AFMT: u32 = 0x61666D74;
pub const SYNC_PACKET_MAGIC_TIME: u32 = 0x74696D65;
pub const SYNC_PACKET_MAGIC_CLOK: u32 = 0x636C6F6B;
pub const SYNC_PACKET_MAGIC_CVRP: u32 = 0x63767270;
pub const SYNC_PACKET_MAGIC_CWPA: u32 = 0x63777061;

pub const ASYN_PACKET_MAGIC_EAT: u32 = 0x65617421;
pub const ASYN_PACKET_MAGIC_FEED: u32 = 0x66656564;
pub const ASYN_PACKET_MAGIC_SPRP: u32 = 0x73707270;
pub const ASYN_PACKET_MAGIC_TJMP: u32 = 0x746A6D70;
pub const ASYN_PACKET_MAGIC_SRAT: u32 = 0x73726174;
pub const ASYN_PACKET_MAGIC_TBAS: u32 = 0x74626173;
pub const ASYN_PACKET_MAGIC_RELS: u32 = 0x72656C73;

pub struct QTPacketCWPA {
    device_clock_ref: u64,
}

fn reply_packet(correlation_id: u64) -> Result<QTPacket, Error> {
    let mut pkt = QTPacket::new();

    match pkt.write_u32(PACKET_MAGIC_REPLY) {
        Err(e) => return Err(e),
        _ => {}
    };

    match pkt.write_u64(correlation_id) {
        Err(e) => return Err(e),
        _ => {}
    };

    match pkt.write_u32(0) {
        Err(e) => return Err(e),
        _ => {}
    };

    Ok(pkt)
}

fn reply_packet_with_clock_ref(correlation_id: u64, clock_ref: u64) -> Result<QTPacket, Error> {
    let mut pkt = match reply_packet(correlation_id) {
        Ok(e) => e,
        Err(e) => return Err(e),
    };

    match pkt.write_u64(clock_ref) {
        Err(e) => return Err(e),
        _ => {}
    };

    Ok(pkt)
}

impl QTPacketCWPA {
    pub fn device_clock_ref(&self) -> u64 {
        self.device_clock_ref
    }

    pub fn from_packet(pkt: &mut QTPacket) -> Result<QTPacketCWPA, Error> {
        // read reversed
        let device_clock_ref = match pkt.read_u64() {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(QTPacketCWPA { device_clock_ref })
    }

    pub fn reply_packet(&self, correlation_id: u64, clock_ref: u64) -> Result<QTPacket, Error> {
        reply_packet_with_clock_ref(correlation_id, clock_ref)
    }
}

pub struct QTPacketASYN {
    sub_type_mark: u32,
    type_header: u64,
    qt_value: Option<QTValue>,
}

impl QTPacketASYN {
    pub fn new(qt_value: Option<QTValue>, sub_type_mark: u32, type_header: u64) -> QTPacketASYN {
        QTPacketASYN {
            sub_type_mark,
            type_header,
            qt_value,
        }
    }

    pub fn as_qt_packet(&mut self) -> Result<QTPacket, Error> {
        let mut pkt = QTPacket::new();
        match pkt.write_u32(PACKET_MAGIC_ASYN) {
            Err(e) => return Err(e),
            _ => {}
        };
        match pkt.write_u64(self.type_header) {
            Err(e) => return Err(e),
            _ => {}
        };
        match pkt.write_u32(self.sub_type_mark) {
            Err(e) => return Err(e),
            _ => {}
        };

        match &mut self.qt_value {
            Some(qt_pkt) => {
                let mut val_pkt = match qt_pkt.as_qt_packet() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let val_pkt_val = match val_pkt.as_bytes() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match pkt.write(val_pkt_val) {
                    Err(e) => return Err(e),
                    _ => {}
                };
            }
            _ => {}
        };

        Ok(pkt)
    }
}

pub struct QTPacketOG {
    unknown: u32,
}

impl QTPacketOG {
    pub fn from_packet(pkt: &mut QTPacket) -> Result<QTPacketOG, Error> {
        // read reversed
        let unknown = match pkt.read_u32() {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(QTPacketOG { unknown })
    }

    pub fn reply_packet(&self, correlation_id: u64) -> Result<QTPacket, Error> {
        let mut pkt = match reply_packet(correlation_id) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match pkt.write_u32(0) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(pkt)
    }
}

pub struct QTPacketCVRP {
    device_clock_ref: u64,
    payload: QTValue,
}

impl QTPacketCVRP {
    pub fn device_clock_ref(&self) -> u64 {
        self.device_clock_ref
    }

    pub fn payload(&self) -> &QTValue {
        &self.payload
    }

    pub fn from_packet(pkt: &mut QTPacket) -> Result<QTPacketCVRP, Error> {
        // read reversed
        let device_clock_ref = match pkt.read_u64() {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        let qt_value = match QTValue::from_qt_packet(pkt) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        Ok(QTPacketCVRP {
            device_clock_ref,
            payload: qt_value,
        })
    }

    pub fn reply_packet(&self, correlation_id: u64, clock_ref: u64) -> Result<QTPacket, Error> {
        reply_packet_with_clock_ref(correlation_id, clock_ref)
    }
}

pub struct QTPacketCLOCK {}

impl QTPacketCLOCK {
    pub fn new() -> QTPacketCLOCK {
        return QTPacketCLOCK {};
    }

    pub fn reply_packet(&self, correlation_id: u64, clock_ref: u64) -> Result<QTPacket, Error> {
        reply_packet_with_clock_ref(correlation_id, clock_ref)
    }
}

pub struct QTPacketTIME {}

impl QTPacketTIME {
    pub fn new() -> QTPacketTIME {
        return QTPacketTIME {};
    }

    pub fn reply_packet(&self, correlation_id: u64, t: Time) -> Result<QTPacket, Error> {
        let mut pkt = match reply_packet(correlation_id) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let t_buffer = match t.as_bytes() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match pkt.write(t_buffer.as_slice()) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(pkt)
    }
}

pub struct QTPacketAFMT {
    audio_desc: AudioStreamDescription,
}

impl QTPacketAFMT {
    pub fn from_packet(pkt: &mut QTPacket) -> Result<QTPacketAFMT, Error> {
        let audio_desc = match AudioStreamDescription::from_qt_packet(pkt) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };
        Ok(QTPacketAFMT { audio_desc })
    }

    pub fn reply_packet(&self, correlation_id: u64) -> Result<QTPacket, Error> {
        let mut pkt = match reply_packet(correlation_id) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let mut arr: Vec<QTValue> = Vec::new();

        arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
            QTValue::StringKey(String::from("Error")),
            QTValue::UInt32(0),
        )));

        let mut val_pkt = match QTValue::Object(arr).as_qt_packet() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let val_pkt_buffer = match val_pkt.as_bytes() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match pkt.write(val_pkt_buffer) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(pkt)
    }
}

pub struct QTPacketSKEW {}

impl QTPacketSKEW {
    pub fn new() -> QTPacketSKEW {
        QTPacketSKEW {}
    }

    pub fn reply_packet(&self, correlation_id: u64, skew: f64) -> Result<QTPacket, Error> {
        let mut pkt = match reply_packet(correlation_id) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match pkt.write_f64(skew) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(pkt)
    }
}

pub struct QTPacketSTOP {}

impl QTPacketSTOP {
    pub fn new() -> QTPacketSTOP {
        QTPacketSTOP {}
    }

    pub fn reply_packet(&self, correlation_id: u64) -> Result<QTPacket, Error> {
        let mut pkt = match reply_packet(correlation_id) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match pkt.write_u32(0) {
            Err(e) => return Err(e),
            _ => {}
        };

        Ok(pkt)
    }
}
