use crate::coremedia::format_desc::FormatDescriptor;
use crate::coremedia::sample::MAGIC_FORMAT_DESCRIPTOR;
use crate::qt_pkt::QTPacket;
use std::fmt::{format, Debug, Formatter, Write};
use std::io::{Error, ErrorKind};

const MAGIC_KEY_VALUE_PAIR: u32 = 0x6B657976; // keyv - vyek
const MAGIC_KEY_STRING: u32 = 0x7374726B; // strk - krts
const MAGIC_KEY_BOOLEAN: u32 = 0x62756C76; // bulv - vlub
const MAGIC_KEY_DICTIONARY: u32 = 0x64696374; // dict - tcid
const MAGIC_KEY_DATA_VALUE: u32 = 0x64617476; // datv - vtad
const MAGIC_KEY_STRING_VALUE: u32 = 0x73747276; // strv - vrts
const MAGIC_KEY_NUMBER_VALUE: u32 = 0x6E6D6276; // nmbv - vbmn
const MAGIC_KEY_IDX: u32 = 0x6964786B;

pub struct QTKeyValuePair {
    key: QTValue,
    value: QTValue,
}

impl QTKeyValuePair {
    pub fn new(key: QTValue, value: QTValue) -> Box<QTKeyValuePair> {
        Box::new(QTKeyValuePair { key, value })
    }

    pub fn key(&self) -> &QTValue {
        &self.key
    }

    pub fn value(&self) -> &QTValue {
        &self.value
    }
}

pub enum QTValue {
    StringKey(String),
    StringValue(String),
    Boolean(bool),
    KeyValuePair(Box<QTKeyValuePair>),
    Object(Vec<QTValue>),
    Float(f64),
    UInt32(u32),
    UInt64(u64),
    Data(Vec<u8>),
    IdxKey(u16),
    FormatDescriptor(Box<FormatDescriptor>),
}

impl QTValue {
    fn get_magic(&self) -> u32 {
        match *self {
            QTValue::StringKey(_) => MAGIC_KEY_STRING,
            QTValue::StringValue(_) => MAGIC_KEY_STRING_VALUE,
            QTValue::Boolean(_) => MAGIC_KEY_BOOLEAN,
            QTValue::KeyValuePair(_) => MAGIC_KEY_VALUE_PAIR,
            QTValue::Object(_) => MAGIC_KEY_DICTIONARY,
            QTValue::Data(_) => MAGIC_KEY_DATA_VALUE,
            QTValue::Float(_) => MAGIC_KEY_NUMBER_VALUE,
            QTValue::UInt32(_) => MAGIC_KEY_NUMBER_VALUE,
            QTValue::UInt64(_) => MAGIC_KEY_NUMBER_VALUE,
            QTValue::IdxKey(_) => MAGIC_KEY_IDX,
            QTValue::FormatDescriptor(_) => MAGIC_FORMAT_DESCRIPTOR,
        }
    }

    pub fn as_qt_packet(&self) -> Result<QTPacket, Error> {
        let mut pkt = QTPacket::new();

        match pkt.write_u32(self.get_magic()) {
            Err(e) => return Err(e),
            _ => {}
        };

        match self {
            QTValue::StringKey(s) => match pkt.write(s.as_bytes()) {
                Err(e) => return Err(e),
                _ => {}
            },
            QTValue::StringValue(s) => match pkt.write(s.as_bytes()) {
                Err(e) => return Err(e),
                _ => {}
            },
            QTValue::Boolean(b) => match b {
                true => match pkt.write(&[1]) {
                    Err(e) => return Err(e),
                    _ => {}
                },
                false => match pkt.write(&[0]) {
                    Err(e) => return Err(e),
                    _ => {}
                },
            },
            QTValue::KeyValuePair(p) => {
                let mut key_buffer = match p.key.as_qt_packet() {
                    Err(e) => return Err(e),
                    Ok(e) => e,
                };

                match pkt.write(match key_buffer.as_bytes() {
                    Err(e) => return Err(e),
                    Ok(e) => e,
                }) {
                    Err(e) => return Err(e),
                    _ => {}
                };

                let mut value_buffer = match p.value.as_qt_packet() {
                    Err(e) => return Err(e),
                    Ok(e) => e,
                };

                match pkt.write(match value_buffer.as_bytes() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                }) {
                    Err(e) => return Err(e),
                    _ => {}
                };
            }
            QTValue::Object(obj) => {
                for o in obj {
                    let mut val_pkt = match o.as_qt_packet() {
                        Ok(e) => e,
                        Err(e) => return Err(e),
                    };

                    let val_pkt_buf = match val_pkt.as_bytes() {
                        Ok(e) => e,
                        Err(e) => return Err(e),
                    };

                    match pkt.write(val_pkt_buf) {
                        Err(e) => return Err(e),
                        _ => {}
                    };
                }
            }
            QTValue::Float(f) => {
                match pkt.write_u8(6) {
                    Err(e) => return Err(e),
                    _ => {}
                };
                match pkt.write_f64(*f) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            QTValue::UInt32(n) => {
                match pkt.write_u8(3) {
                    Err(e) => return Err(e),
                    _ => {}
                };
                match pkt.write_u32(*n) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            QTValue::UInt64(n) => {
                match pkt.write_u8(4) {
                    Err(e) => return Err(e),
                    _ => {}
                };
                match pkt.write_u64(*n) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            QTValue::Data(u) => match pkt.write(u.as_slice()) {
                Err(e) => return Err(e),
                _ => {}
            },
            QTValue::FormatDescriptor(d) => {
                let mut fd_pkt = match d.as_qt_packet() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let fd_buffer = match fd_pkt.as_bytes() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match pkt.write(fd_buffer) {
                    Err(e) => return Err(e),
                    _ => {}
                };
            }
            QTValue::IdxKey(i) => match pkt.write_u16(*i) {
                Err(e) => return Err(e),
                _ => {}
            },
        };

        Ok(pkt)
    }

    pub fn from_qt_packet(pkt: &mut QTPacket) -> Result<QTValue, Error> {
        let pkt_len = match pkt.read_u32() {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        let magic = match pkt.read_u32() {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        let obj_val = match magic {
            MAGIC_KEY_VALUE_PAIR => Some(QTValue::KeyValuePair(Box::new(QTKeyValuePair {
                key: match QTValue::from_qt_packet(pkt) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                },
                value: match QTValue::from_qt_packet(pkt) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                },
            }))),
            MAGIC_KEY_DICTIONARY => {
                // create new qt packet
                let mut obj_pkt = match QTPacket::read_qt_packet(pkt, pkt_len as usize - 8) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let mut arr: Vec<QTValue> = Vec::new();
                loop {
                    match QTValue::from_qt_packet(&mut obj_pkt) {
                        Ok(mut e) => {
                            let mut brow = &mut e;
                            let mut wrap_pkt = brow.as_qt_packet().expect("as_qt_packet");
                            let buf = wrap_pkt.as_bytes().expect("as bytes");
                            arr.push(e)
                        }
                        Err(e) => match e.kind() {
                            ErrorKind::UnexpectedEof => break,
                            _ => return Err(e),
                        },
                    }
                }

                Some(QTValue::Object(arr))
            }
            MAGIC_FORMAT_DESCRIPTOR => match FormatDescriptor::from_qt_packet(pkt) {
                Ok(e) => Some(QTValue::FormatDescriptor(Box::new(e))),
                Err(e) => return Err(e),
            },
            _ => None,
        };

        if obj_val.is_some() {
            return Ok(obj_val.unwrap());
        }

        let mut data: Vec<u8> = vec![0; pkt_len as usize - 8];
        match pkt.read_exact(&mut data) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match magic {
            MAGIC_KEY_STRING => Ok(QTValue::StringKey(match String::from_utf8(data) {
                Ok(e) => e,
                Err(e) => return Err(Error::new(ErrorKind::InvalidData, "string utf8")),
            })),
            MAGIC_KEY_STRING_VALUE => Ok(QTValue::StringKey(match String::from_utf8(data) {
                Ok(e) => e,
                Err(e) => return Err(Error::new(ErrorKind::InvalidData, "string utf8")),
            })),
            MAGIC_KEY_BOOLEAN => match data[0] {
                0 => Ok(QTValue::Boolean(false)),
                1 => Ok(QTValue::Boolean(true)),
                _ => return Err(Error::new(ErrorKind::InvalidData, "boolean overflow")),
            },
            MAGIC_KEY_DATA_VALUE => Ok(QTValue::Data(data)),
            MAGIC_KEY_NUMBER_VALUE => match data[0] {
                6 => Ok(QTValue::Float(f64::from_le_bytes([
                    data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
                ]))),
                5 => Ok(QTValue::UInt32(u32::from_le_bytes([
                    data[1], data[2], data[3], data[4],
                ]))),
                4 => Ok(QTValue::UInt64(u64::from_le_bytes([
                    data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
                ]))),
                3 => Ok(QTValue::UInt32(u32::from_le_bytes([
                    data[1], data[2], data[3], data[4],
                ]))),
                _ => return Err(Error::new(ErrorKind::InvalidData, "unknown number spec")),
            },
            MAGIC_KEY_IDX => Ok(QTValue::IdxKey(u16::from_le_bytes([data[0], data[1]]))),
            _ => return Err(Error::new(ErrorKind::InvalidData, "unknown magic")),
        }
    }

    pub fn to_str(&self, ident: String) -> String {
        match self {
            QTValue::StringKey(s) => format!("{}StringKey={}", ident, s.as_str()),
            QTValue::StringValue(s) => format!("{}StringValue={}", ident, s.as_str()),
            QTValue::Boolean(b) => format!("{}Boolean={}", ident, b),
            QTValue::KeyValuePair(kv) => format!(
                "{}KeyValuePair(\n{}  Key: {},\n{}  Value: {},\n{})",
                ident,
                ident,
                kv.key.to_str(String::from(&ident)),
                ident,
                kv.value.to_str(String::from(&ident)),
                ident
            ),
            QTValue::Object(o) => {
                let mut str = format!("{}Object(\n", ident);
                for v in o {
                    let mut si = String::from(ident.as_str());
                    si.push_str("    ");
                    str += format!("{}\n", v.to_str(si)).as_str()
                }
                str += format!("{}  )", ident).as_str();
                str
            }
            QTValue::Data(d) => format!("{}Data={}", ident, hex::encode(d)),
            QTValue::Float(f) => format!("{}Float={}", ident, f),
            QTValue::UInt32(i) => format!("{}UInt32={}", ident, i),
            QTValue::UInt64(i) => format!("{}UInt64={}", ident, i),
            QTValue::IdxKey(i) => format!("{}IdxKey={}", ident, i),
            QTValue::FormatDescriptor(fd) => format!("{}FormatDescriptor=...", ident),
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            QTValue::StringKey(s) => Some(String::from(s)),
            QTValue::StringValue(s) => Some(String::from(s)),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            QTValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_pair(&self) -> Option<&QTKeyValuePair> {
        match self {
            QTValue::KeyValuePair(kv) => Some(kv),
            _ => None,
        }
    }

    pub fn as_vec(&self) -> Option<&Vec<QTValue>> {
        match self {
            QTValue::Object(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            QTValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            QTValue::UInt64(u) => Some(*u),
            _ => None,
        }
    }

    pub fn as_idx(&self) -> Option<u16> {
        match self {
            QTValue::IdxKey(u) => Some(*u),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        match self {
            QTValue::UInt32(u) => Some(*u),
            _ => None,
        }
    }

    pub fn as_data(&self) -> Option<&Vec<u8>> {
        match self {
            QTValue::Data(data) => Some(data),
            _ => None,
        }
    }
}

impl Debug for QTValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str(String::from("")).as_str())
            .expect("write fmt");
        f.write_str("\n")
    }
}
