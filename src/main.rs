#![allow(dead_code)]

extern crate core;

mod apple;
mod coremedia;
mod qt;
mod qt_device;
mod qt_pkt;
mod qt_value;

use crate::coremedia::sample::{SampleBuffer, MEDIA_TYPE_VIDEO};
use crate::qt::QuickTime;
use byteorder::{BigEndian, WriteBytesExt};
use rusty_libimobiledevice::error::IdeviceError;
use rusty_libimobiledevice::idevice;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{mpsc, Arc};
use std::{io, thread};

fn get_apple_device() -> Result<idevice::Device, IdeviceError> {
    let devices = match idevice::get_devices() {
        Ok(d) => d,
        Err(e) => return Err(e),
    };

    for device in devices {
        if device.get_network() {
            continue;
        }

        return Ok(device);
    }

    return Err(IdeviceError::NoDevice);
}

fn main() {
    let device = match get_apple_device() {
        Ok(d) => d,
        Err(e) => {
            println!("get_apple_device: {:?}", e);
            return;
        }
    };

    let lockdownd = match device.new_lockdownd_client("qtstream") {
        Ok(client) => client,
        Err(e) => {
            println!("new_lockdownd_client: {:?}", e);
            return;
        }
    };

    let sn = match lockdownd.get_device_udid() {
        Ok(sn) => sn,
        Err(e) => {
            println!("get_device_udid: {:?}", e);
            return;
        }
    };

    let usb_device = match apple::get_usb_device(sn.replace("-", "").as_str()) {
        Ok(d) => d,
        Err(e) => {
            println!("libusb: {:?}", e);
            return;
        }
    };

    let (tx, rx): (
        SyncSender<Result<SampleBuffer, io::Error>>,
        Receiver<Result<SampleBuffer, io::Error>>,
    ) = mpsc::sync_channel(256);

    let mut qt = QuickTime::new(usb_device, tx);

    match qt.init() {
        Err(e) => {
            println!("init qt failed {}", e);
            return;
        }
        _ => {}
    }

    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&qt.term()))
        .expect("register hook failed");

    let t = thread::spawn(move || {
        match qt.run() {
            Err(e) => {
                println!("quick time loop exit: {}", e)
            }
            _ => {}
        };
    });

    let mut file = File::create("record.h264").expect("file");

    loop {
        let message = rx.recv().expect("read packet from channel");
        if message.is_err() {
            break;
        }

        let sample_buffer = message.unwrap();

        if sample_buffer.media_type() == MEDIA_TYPE_VIDEO {
            match sample_buffer.format_description() {
                Some(fd) => {
                    file.write_u32::<BigEndian>(1).expect("write nalu magic");
                    file.write(fd.avc1().sps()).expect("write sps");
                    file.write_u32::<BigEndian>(1).expect("write nalu magic");
                    file.write(fd.avc1().pps()).expect("write pps");
                }
                None => {}
            };
            match sample_buffer.sample_data() {
                Some(buf) => {
                    let mut cur = buf;
                    while cur.len() > 0 {
                        let slice_len =
                            u32::from_be_bytes([cur[0], cur[1], cur[2], cur[3]]) as usize;
                        file.write_u32::<BigEndian>(1).expect("write nalu magic");
                        file.write(&cur[4..slice_len + 4]).expect("write sdat");
                        cur = &cur[slice_len + 4..];
                    }
                }
                None => {}
            };
        }
    }

    file.flush().expect("flush");

    t.join().expect("loop thread term");
}
