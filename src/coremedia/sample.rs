use crate::coremedia::format_desc::FormatDescriptor;
use crate::coremedia::time::Time;
use crate::qt_pkt::QTPacket;
use crate::qt_value::QTValue;
use std::fmt::{Debug, Formatter};
use std::io::Error;

pub const MAGIC_AUDIO_STREAM_DESCRIPTION: u32 = 0x61736264;
pub const MAGIC_FORMAT_DESCRIPTOR: u32 = 0x66647363;
pub const MAGIC_VIDEO_DIMENSION: u32 = 0x7664696D;
pub const MAGIC_EXTENSION: u32 = 0x6578746E;
pub const MAGIC_MEDIA_TYPE: u32 = 0x6D646961;
pub const MAGIC_CODEC: u32 = 0x636F6463;
pub const MEDIA_TYPE_VIDEO: u32 = 0x76696465;
pub const MEDIA_TYPE_SOUND: u32 = 0x736F756E;
pub const CODEC_AVC1: u32 = 0x61766331;

pub struct SampleTimingInfo {
    duration: Time,
    presentation_time_stamp: Time,
    decode_time_stamp: Time,
}

impl SampleTimingInfo {
    pub fn from_qt_packet(pkt: &mut QTPacket) -> SampleTimingInfo {
        SampleTimingInfo {
            duration: Time::from_qt_packet(pkt),
            presentation_time_stamp: Time::from_qt_packet(pkt),
            decode_time_stamp: Time::from_qt_packet(pkt),
        }
    }
}

impl Debug for SampleTimingInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("sample_timing_info:\n").expect("write");
        f.write_fmt(format_args!("duration: \n{:?}\n", self.duration))
            .expect("write");
        f.write_fmt(format_args!(
            "presentation_time_stamp: \n{:?}\n",
            self.presentation_time_stamp
        ))
        .expect("write");
        f.write_fmt(format_args!(
            "decode_time_stamp: \n{:?}\n",
            self.decode_time_stamp
        ))
    }
}

pub struct SampleBuffer {
    output_presentation_time_stamp: Option<Time>,
    format_description: Option<FormatDescriptor>,
    num_samples: u32,                                        //nsmp
    sample_timing_info_array: Option<Vec<SampleTimingInfo>>, //stia
    sample_data: Option<Vec<u8>>,
    sample_sizes: Option<Vec<u32>>,
    attachments: Option<Vec<QTValue>>, //satt
    sary: Option<Vec<QTValue>>,        //sary
    media_type: u32,
}

const SBUF: u32 = 0x73627566; //the cmsamplebuf and only content of feed asyns
const OPTS: u32 = 0x6F707473; //output presentation timestamp?
const STIA: u32 = 0x73746961; //sampleTimingInfoArray
const SDAT: u32 = 0x73646174; //the nalu
const SATT: u32 = 0x73617474; //indexkey dict with only number values, CMSampleBufferGetSampleAttachmentsArray
const SARY: u32 = 0x73617279; //some dict with index and one boolean
const SSIZ: u32 = 0x7373697A; //samplesize in bytes, size of what is contained in sdat, sample size array i think
const NSMP: u32 = 0x6E736D70; //numsample so you know how many things are in the arrays
const FREE: u32 = 0x66726565;

impl SampleBuffer {
    pub fn new(media_type: u32) -> SampleBuffer {
        SampleBuffer {
            media_type,
            sary: None,
            attachments: None,
            sample_sizes: None,
            sample_data: None,
            sample_timing_info_array: None,
            num_samples: 0,
            format_description: None,
            output_presentation_time_stamp: None,
        }
    }

    pub fn sary(&self) -> &Vec<QTValue> {
        self.sary.as_ref().expect("take sary")
    }

    pub fn sample_data(&self) -> Option<&[u8]> {
        match &self.sample_data {
            Some(e) => Some(e.as_slice()),
            None => None,
        }
    }

    pub fn format_description(&self) -> Option<&FormatDescriptor> {
        match &self.format_description {
            Some(e) => Some(e),
            None => None,
        }
    }

    pub fn media_type(&self) -> u32 {
        self.media_type
    }

    pub fn output_presentation_time_stamp(&self) -> Option<Time> {
        self.output_presentation_time_stamp.clone()
    }

    pub fn from_qt_packet(pkt: &mut QTPacket, media_type: u32) -> Result<SampleBuffer, Error> {
        let mut sample = Self::new(media_type);

        let (mut sbuf, _) =
            QTPacket::from_qt_packet_with_magic(pkt, SBUF).expect("read sbuf packet");

        while sbuf.pos() < sbuf.len().expect("sbuf length") {
            let (mut inner, magic) = match sbuf.read_qt_packet_with_magic() {
                Ok(e) => e,
                Err(e) => return Err(e),
            };

            match magic {
                OPTS => {
                    sample.output_presentation_time_stamp = Some(Time::from_qt_packet(&mut inner))
                }
                STIA => {
                    let mut arr: Vec<SampleTimingInfo> = Vec::new();
                    while inner.pos() < inner.len().expect("sita length") {
                        arr.push(SampleTimingInfo::from_qt_packet(&mut inner))
                    }
                    sample.sample_timing_info_array = Some(arr);
                }
                SDAT => {
                    let inner_len = inner.len().expect("inner length");
                    let mut sample_data: Vec<u8> = vec![0; inner_len as usize - 8];
                    inner.read(&mut sample_data).expect("sdat read sample data");
                    sample.sample_data = Some(sample_data);
                }
                NSMP => sample.num_samples = inner.read_u32().expect("nsmp read sample length"),
                SSIZ => {
                    let mut arr: Vec<u32> = Vec::new();
                    while inner.pos() < inner.len().expect("ssiz length") {
                        arr.push(inner.read_u32().expect("read ssiz"))
                    }
                    sample.sample_sizes = Some(arr);
                }
                MAGIC_FORMAT_DESCRIPTOR => {
                    sample.format_description = Some(
                        FormatDescriptor::from_qt_packet(&mut inner)
                            .expect("read format descriptor"),
                    )
                }
                SATT => {
                    let mut arr: Vec<QTValue> = Vec::new();
                    while inner.pos() < inner.len().expect("satt length") {
                        arr.push(QTValue::from_qt_packet(&mut inner).expect("read satt"))
                    }
                    sample.attachments = Some(arr);
                }
                SARY => {
                    let mut arr: Vec<QTValue> = Vec::new();
                    while inner.pos() < inner.len().expect("sary length") {
                        arr.push(QTValue::from_qt_packet(&mut inner).expect("read sary"))
                    }
                    sample.sary = Some(arr);
                }
                FREE => {
                    // free box
                }
                _ => {
                    println!(
                        "invalid data {}",
                        format!("sbuf invalid magic {:#x}", magic)
                    );
                }
            };
        }

        Ok(sample)
    }
}

impl Debug for SampleBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("SampleBuffer:\n").expect("write");
        if self.output_presentation_time_stamp.is_some() {
            f.write_fmt(format_args!(
                "output_presentation_time_stamp: \n{:?}\n",
                self.output_presentation_time_stamp().as_ref().unwrap()
            ))
            .expect("write");
        }
        f.write_fmt(format_args!("num_samples: {}\n", self.num_samples))
            .expect("write");
        if self.sample_timing_info_array.is_some() {
            for timing in self.sample_timing_info_array.as_ref().unwrap() {
                f.write_fmt(format_args!("sample_timing_info_array:\n {:?}\n", timing))
                    .expect("write");
            }
        }
        f.write_fmt(format_args!(
            "sample_data: {}\n",
            self.sample_data.is_some()
        ))
        .expect("write");
        f.write_fmt(format_args!("sample_sizes: {:?}\n", self.sample_sizes))
            .expect("write");
        if self.attachments.is_some() {
            let mut i = 0;
            for qtv in self.attachments.as_ref().unwrap() {
                f.write_fmt(format_args!("attachments.{}:\n{:?}\n", i, qtv))
                    .expect("write");
                i += 1;
            }
        }
        if self.sary.is_some() {
            let mut i = 0;
            for qtv in self.sary.as_ref().unwrap() {
                f.write_fmt(format_args!("sary.{}:\n{:?}\n", i, qtv))
                    .expect("write");
                i += 1;
            }
        }
        f.write_str("-----")
    }
}
