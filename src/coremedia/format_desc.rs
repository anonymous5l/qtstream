use crate::coremedia::audio_desc::AudioStreamDescription;
use crate::coremedia::sample::{
    MAGIC_AUDIO_STREAM_DESCRIPTION, MAGIC_CODEC, MAGIC_EXTENSION, MAGIC_MEDIA_TYPE,
    MAGIC_VIDEO_DIMENSION, MEDIA_TYPE_SOUND, MEDIA_TYPE_VIDEO,
};
use crate::qt_pkt::QTPacket;
use crate::qt_value::QTValue;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::{Error, ErrorKind};

pub struct FormatDescriptor {
    media_type: u32,
    video_dimension_width: u32,
    video_dimension_height: u32,
    codec: u32,
    extensions: Option<Vec<QTValue>>,
    //PPS contains bytes of the Picture Parameter Set h264 NALu
    pps: Option<Vec<u8>>,
    //SPS contains bytes of the Picture Parameter Set h264 NALu
    sps: Option<Vec<u8>>,
    audio_stream_basic_description: Option<AudioStreamDescription>,
}

impl FormatDescriptor {
    pub fn pps(&self) -> &[u8] {
        let pps = self.pps.as_ref().expect("pps");
        pps.as_slice()
    }

    pub fn sps(&self) -> &[u8] {
        let sps = self.sps.as_ref().expect("pps");
        sps.as_slice()
    }

    pub fn from_qt_packet(pkt: &mut QTPacket) -> Result<FormatDescriptor, io::Error> {
        println!("format descriptor");
        let (mut mdia_pkt, _) = match QTPacket::from_qt_packet_with_magic(pkt, MAGIC_MEDIA_TYPE) {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let media_type = match mdia_pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match media_type {
            MEDIA_TYPE_SOUND => {
                let (mut asdb, _) = match QTPacket::from_qt_packet_with_magic(
                    pkt,
                    MAGIC_AUDIO_STREAM_DESCRIPTION,
                ) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let asd = match AudioStreamDescription::from_qt_packet(&mut asdb) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                Ok(FormatDescriptor {
                    media_type: MEDIA_TYPE_SOUND,
                    video_dimension_width: 0,
                    video_dimension_height: 0,
                    codec: 0,
                    extensions: None,
                    pps: None,
                    sps: None,
                    audio_stream_basic_description: Some(asd),
                })
            }
            MEDIA_TYPE_VIDEO => {
                let (mut video_dimension, _) =
                    match QTPacket::from_qt_packet_with_magic(pkt, MAGIC_VIDEO_DIMENSION) {
                        Ok(e) => e,
                        Err(e) => return Err(e),
                    };

                let video_width = match video_dimension.read_u32() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let video_height = match video_dimension.read_u32() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let (mut codec_pkt, _) = match QTPacket::from_qt_packet_with_magic(pkt, MAGIC_CODEC)
                {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let codec = match codec_pkt.read_u32() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let (mut extension_pkt, _) =
                    match QTPacket::from_qt_packet_with_magic(pkt, MAGIC_EXTENSION) {
                        Ok(e) => e,
                        Err(e) => return Err(e),
                    };

                let mut extensions: Vec<QTValue> = Vec::new();

                let mut pps: Option<Vec<u8>> = None;
                let mut sps: Option<Vec<u8>> = None;

                loop {
                    let extension = match QTValue::from_qt_packet(&mut extension_pkt) {
                        Ok(e) => e,
                        Err(e) => match e.kind() {
                            ErrorKind::UnexpectedEof => break,
                            _ => return Err(e),
                        },
                    };

                    println!("video format descriptor {:?}", extension);

                    match extension.as_pair() {
                        Some(kv) => match kv.key().as_idx() {
                            Some(idx) => match idx {
                                49 => {
                                    let obj = kv.value().as_vec().expect("idx 49 is not object");
                                    if obj.len() > 0 {
                                        let obj_kv =
                                            obj[0].as_pair().expect("obj[0] is not kv pair");
                                        let obj_k =
                                            obj_kv.key().as_idx().expect("obj[0].key is not idx");
                                        if obj_k == 105 {
                                            let obj_data = obj_kv
                                                .value()
                                                .as_data()
                                                .expect("obj[0].value is not data");

                                            let pps_len = obj_data[7] as usize;
                                            pps = Some(Vec::from(&obj_data[8..8 + pps_len]));
                                            let sps_len = obj_data[10 + pps_len] as usize;
                                            sps = Some(Vec::from(
                                                &obj_data[11 + pps_len..11 + pps_len + sps_len],
                                            ));
                                        }
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        },
                        _ => {}
                    }

                    extensions.push(extension);
                }

                Ok(FormatDescriptor {
                    media_type: MEDIA_TYPE_VIDEO,
                    video_dimension_width: video_width,
                    video_dimension_height: video_height,
                    codec,
                    extensions: Some(extensions),
                    pps,
                    sps,
                    audio_stream_basic_description: None,
                })
            }
            _ => return Err(Error::new(ErrorKind::InvalidData, "media type invalid")),
        }
    }

    pub fn as_qt_packet(&self) -> Result<QTPacket, io::Error> {
        let mut mdia_pkt = QTPacket::new();
        match mdia_pkt.write_u32(MAGIC_MEDIA_TYPE) {
            Err(e) => return Err(e),
            _ => {}
        };

        match mdia_pkt.write_u32(self.media_type) {
            Err(e) => return Err(e),
            _ => {}
        };

        match self.media_type {
            MEDIA_TYPE_SOUND => {
                let mut asdb = QTPacket::new_with_magic(MAGIC_AUDIO_STREAM_DESCRIPTION);

                let buffer = match self
                    .audio_stream_basic_description
                    .as_ref()
                    .unwrap()
                    .as_buffer()
                {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let asdb_buffer = match asdb.write(buffer.as_slice()) {
                    Err(e) => return Err(e),
                    Ok(_) => match asdb.as_bytes() {
                        Ok(e) => e,
                        Err(e) => return Err(e),
                    },
                };

                match mdia_pkt.write(asdb_buffer) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            MEDIA_TYPE_VIDEO => {
                let mut vd_pkt = QTPacket::new_with_magic(MAGIC_VIDEO_DIMENSION);

                match vd_pkt.write_u32(self.video_dimension_width) {
                    Err(e) => return Err(e),
                    _ => {}
                };

                match vd_pkt.write_u32(self.video_dimension_height) {
                    Err(e) => return Err(e),
                    _ => {}
                };

                let mut codec_pkt = QTPacket::new_with_magic(MAGIC_CODEC);

                match codec_pkt.write_u32(self.codec) {
                    Err(e) => return Err(e),
                    _ => {}
                };

                let codec_buffer = match codec_pkt.as_bytes() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match vd_pkt.write(codec_buffer) {
                    Err(e) => return Err(e),
                    _ => {}
                };

                let mut extension_pkt = QTPacket::new_with_magic(MAGIC_EXTENSION);

                if self.extensions.is_some() {
                    for extension in self.extensions.as_ref().unwrap() {
                        let mut ext_val_pkt = match extension.as_qt_packet() {
                            Ok(e) => e,
                            Err(e) => return Err(e),
                        };

                        let extensions_buffer = match ext_val_pkt.as_bytes() {
                            Ok(e) => e,
                            Err(e) => return Err(e),
                        };

                        match extension_pkt.write(extensions_buffer) {
                            Err(e) => return Err(e),
                            _ => {}
                        };
                    }

                    let mut extension_buffer = match extension_pkt.as_bytes() {
                        Err(e) => return Err(e),
                        Ok(e) => e,
                    };

                    let mut extension_buffer_vec = Vec::from(extension_buffer);

                    extension_buffer_vec[0] = 0;
                    extension_buffer_vec[1] = 0;
                    extension_buffer_vec[2] = 0;
                    extension_buffer_vec[3] = 0;

                    match vd_pkt.write(extension_buffer_vec.as_slice()) {
                        Err(e) => return Err(e),
                        _ => {}
                    };
                }

                let vd_buffer = match vd_pkt.as_bytes() {
                    Err(e) => return Err(e),
                    Ok(e) => e,
                };

                match mdia_pkt.write(vd_buffer) {
                    Err(e) => return Err(e),
                    _ => {}
                };
            }
            _ => return Err(Error::new(ErrorKind::InvalidData, "media type invalid")),
        };

        Ok(mdia_pkt)
    }
}

impl Debug for FormatDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Format Descriptor")
    }
}
