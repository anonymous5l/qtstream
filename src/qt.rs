use crate::apple::AppleDevice;
use crate::coremedia::clock::Clock;
use crate::coremedia::sample::{SampleBuffer, MEDIA_TYPE_SOUND, MEDIA_TYPE_VIDEO};
use crate::coremedia::time::Time;
use crate::qt_device::{qt_hpa1_device_info, qt_hpd1_device_info};
use crate::qt_pkt;
use crate::qt_pkt::{
    QTPacket, QTPacketAFMT, QTPacketASYN, QTPacketCLOCK, QTPacketSKEW, QTPacketSTOP, QTPacketTIME,
};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{BufRead, Cursor, Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

pub struct QuickTime {
    device: AppleDevice,
    term: Arc<AtomicBool>,
    clock: Option<Clock>,
    need_clock_ref: Option<u64>,
    local_audio_clock: Option<Clock>,
    device_audio_clock: Option<u64>,
    start_time_local_audio_clock: Option<Time>,
    last_eat_frame_received_local_audio_clock: Option<Time>,
    start_time_device_audio_clock: Option<Time>,
    last_eat_frame_received_device_audio_clock: Option<Time>,
    packet_pool: Cursor<Vec<u8>>,
    tx: SyncSender<Result<SampleBuffer, Error>>,
}

const HPD1: u32 = 0x68706431;
const HPA1: u32 = 0x68706131;
const HPD0: u32 = 0x68706430;
const HPA0: u32 = 0x68706130;
const NEED: u32 = 0x6E656564;
const EMPTY_CF_TYPE: u64 = 1;

impl AsRef<QuickTime> for QuickTime {
    fn as_ref(&self) -> &QuickTime {
        self
    }
}

impl QuickTime {
    pub fn new(device: AppleDevice, tx: SyncSender<Result<SampleBuffer, Error>>) -> QuickTime {
        // let (close_tx, close_rx): (Sender<()>, Receiver<()>) = mpsc::channel();

        return QuickTime {
            device,
            term: Arc::new(AtomicBool::new(false)),
            clock: None,
            need_clock_ref: None,
            local_audio_clock: None,
            device_audio_clock: None,
            start_time_local_audio_clock: None,
            last_eat_frame_received_local_audio_clock: None,
            start_time_device_audio_clock: None,
            last_eat_frame_received_device_audio_clock: None,
            packet_pool: Cursor::new(Vec::new()),
            tx,
            // close_tx,
            // close_rx,
        };
    }

    pub fn term(&self) -> &Arc<AtomicBool> {
        return &self.term;
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.device.set_qt_enabled(true).expect("set qt enabled");

        match self.device.claim_interface() {
            Some(_) => return Err(Error::new(ErrorKind::Other, "claim interface")),
            _ => {}
        };

        match self.device.init_bulk_endpoint() {
            Some(_) => return Err(Error::new(ErrorKind::Other, "init bulk endpoint")),
            _ => {}
        };

        match self.device.clear_feature() {
            Some(_) => return Err(Error::new(ErrorKind::Other, "clear feature")),
            _ => {}
        };

        Ok(())
    }

    fn read(&mut self) -> Result<Option<QTPacket>, Error> {
        let mut buffer: Vec<u8> = vec![0; self.device.max_read_packet_size() as usize];
        let buffer_size = match self.device.read_bulk(&mut buffer) {
            Ok(e) => e,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    format!("read bulk {}", e),
                ))
            }
        };

        if buffer_size <= 0 {
            return Ok(None);
        }

        self.packet_pool
            .seek(SeekFrom::End(0))
            .expect("packet pool seek to end");

        match self.packet_pool.write(&buffer[..buffer_size]) {
            Err(e) => return Err(e),
            _ => {}
        };

        self.packet_pool
            .seek(SeekFrom::Start(0))
            .expect("packet pool seek to start");

        let pkt_len = match self.packet_pool.read_u32::<LittleEndian>() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        let pool_len = self
            .packet_pool
            .seek(SeekFrom::End(0))
            .expect("packet pool seek to end");

        if pool_len >= pkt_len as u64 {
            self.packet_pool
                .seek(SeekFrom::Start(0))
                .expect("packet pool seek to start");

            let mut pkt_buffer: Vec<u8> = vec![0; pkt_len as usize];
            self.packet_pool
                .read_exact(&mut pkt_buffer)
                .expect("packet pool read");

            let pkt = QTPacket::from_bytes(&pkt_buffer).expect("qt packet from bytes");

            let remain = self.packet_pool.fill_buf().expect("remain");

            self.packet_pool = Cursor::new(Vec::from(remain));

            return Ok(Some(pkt));
        }

        Ok(None)
    }

    fn write(&self, data: &mut QTPacket) -> Result<usize, Error> {
        let buf = match data.as_bytes() {
            Ok(d) => d,
            Err(_) => return Err(Error::new(ErrorKind::InvalidData, "packet as_bytes")),
        };

        match self.device.write_bulk(buf) {
            Ok(e) => Ok(e),
            Err(e) => Err(Error::new(
                ErrorKind::BrokenPipe,
                format!("write bulk {}", e),
            )),
        }
    }

    fn handle_pkt(&mut self, pkt: &mut QTPacket, sync: bool) -> Result<(), Error> {
        let clock_ref = match pkt.read_u64() {
            Err(e) => return Err(e),
            Ok(e) => e,
        };

        let magic = match pkt.read_u32() {
            Ok(e) => e,
            Err(e) => return Err(e),
        };

        match sync {
            true => {
                let correlation_id = match pkt.read_u64() {
                    Ok(m) => m,
                    Err(e) => return Err(e),
                };
                self.handle_sync_pkt(pkt, clock_ref, magic, correlation_id)
            }
            false => self.handle_asyn_pkt(pkt, clock_ref, magic),
        }
    }

    fn handle_sync_pkt(
        &mut self,
        pkt: &mut QTPacket,
        clock_ref: u64,
        magic: u32,
        correlation_id: u64,
    ) -> Result<(), Error> {
        match magic {
            qt_pkt::SYNC_PACKET_MAGIC_OG => {
                let og_pkt = match qt_pkt::QTPacketOG::from_packet(pkt) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let mut reply_packet = match og_pkt.reply_packet(correlation_id) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match self.write(&mut reply_packet) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            qt_pkt::SYNC_PACKET_MAGIC_CWPA => {
                let cwpa_pkt = match qt_pkt::QTPacketCWPA::from_packet(pkt) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let device_clock_ref = cwpa_pkt.device_clock_ref() + 1000;

                self.local_audio_clock = Some(Clock::new_with_host_time(device_clock_ref));

                self.device_audio_clock = Some(cwpa_pkt.device_clock_ref());

                let display_device_info = qt_hpd1_device_info();
                let audio_device_info = qt_hpa1_device_info();

                let mut display_pkt =
                    match QTPacketASYN::new(Some(display_device_info), HPD1, EMPTY_CF_TYPE)
                        .as_qt_packet()
                    {
                        Ok(e) => e,
                        Err(e) => return Err(e),
                    };

                match self.write(&mut display_pkt) {
                    Err(e) => return Err(e),
                    _ => {}
                }

                let mut reply_packet = match cwpa_pkt.reply_packet(correlation_id, device_clock_ref)
                {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let display_pkt_buf = match display_pkt.as_bytes() {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match reply_packet.write(display_pkt_buf) {
                    Err(e) => return Err(e),
                    _ => {}
                };

                match self.write(&mut reply_packet) {
                    Err(e) => return Err(e),
                    _ => {}
                }

                let mut audio_pkt = match QTPacketASYN::new(
                    Some(audio_device_info),
                    HPA1,
                    cwpa_pkt.device_clock_ref(),
                )
                .as_qt_packet()
                {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match self.write(&mut audio_pkt) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            qt_pkt::SYNC_PACKET_MAGIC_CVRP => {
                let cvrp_pkt = match qt_pkt::QTPacketCVRP::from_packet(pkt) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                self.need_clock_ref = Some(cvrp_pkt.device_clock_ref());

                let mut need_pkt = match QTPacketASYN::new(None, NEED, cvrp_pkt.device_clock_ref())
                    .as_qt_packet()
                {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match self.write(&mut need_pkt) {
                    Err(e) => return Err(e),
                    _ => {}
                }

                let device_clock_ref = cvrp_pkt.device_clock_ref() + 0x1000AF;

                let mut reply_packet = match cvrp_pkt.reply_packet(correlation_id, device_clock_ref)
                {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match self.write(&mut reply_packet) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            qt_pkt::SYNC_PACKET_MAGIC_CLOK => {
                let host_time = clock_ref + 0x10000;

                self.clock = Some(Clock::new_with_host_time(host_time));

                let mut reply_packet =
                    match QTPacketCLOCK::new().reply_packet(correlation_id, host_time) {
                        Err(e) => return Err(e),
                        Ok(e) => e,
                    };

                match self.write(&mut reply_packet) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            qt_pkt::SYNC_PACKET_MAGIC_TIME => {
                QTPacketTIME::new()
                    .reply_packet(
                        correlation_id,
                        self.clock.as_ref().expect("clock none").get_time(),
                    )
                    .expect("qt packet time reply");
            }
            qt_pkt::SYNC_PACKET_MAGIC_AFMT => {
                let afmt_pkt = match QTPacketAFMT::from_packet(pkt) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let mut reply_packet = match afmt_pkt.reply_packet(correlation_id) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match self.write(&mut reply_packet) {
                    Err(e) => return Err(e),
                    _ => {}
                }
            }
            qt_pkt::SYNC_PACKET_MAGIC_SKEW => {
                let stlac = self
                    .start_time_local_audio_clock
                    .as_ref()
                    .expect("start_time_local_audio_clock None");

                let stdac = self
                    .start_time_device_audio_clock
                    .as_ref()
                    .expect("start_time_device_audio_clock None");

                let lefrlac = self
                    .last_eat_frame_received_local_audio_clock
                    .as_ref()
                    .expect("last_eat_frame_received_local_audio_clock None");

                let lefrdac = self
                    .last_eat_frame_received_device_audio_clock
                    .as_ref()
                    .expect("last_eat_frame_received_device_audio_clock None");

                let skew = Clock::calculate_skew(stlac, lefrlac, stdac, lefrdac);

                let mut pkt = match QTPacketSKEW::new().reply_packet(correlation_id, skew) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match self.write(&mut pkt) {
                    Err(e) => return Err(e),
                    _ => {}
                };
            }
            qt_pkt::SYNC_PACKET_MAGIC_STOP => {
                let mut pkt = match QTPacketSTOP::new().reply_packet(correlation_id) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match self.write(&mut pkt) {
                    Err(e) => return Err(e),
                    _ => {}
                };
            }
            _ => {
                println!("SYNC_UNKNOWN_MAGIC - {}", magic);
            }
        };

        Ok(())
    }

    fn handle_asyn_pkt(
        &mut self,
        pkt: &mut QTPacket,
        _clock_ref: u64,
        magic: u32,
    ) -> Result<(), Error> {
        match magic {
            qt_pkt::ASYN_PACKET_MAGIC_EAT => {
                let sample_buffer = match SampleBuffer::from_qt_packet(pkt, MEDIA_TYPE_SOUND) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                if self.last_eat_frame_received_device_audio_clock.is_none() {
                    self.start_time_device_audio_clock =
                        sample_buffer.output_presentation_time_stamp();
                    self.start_time_local_audio_clock = Some(
                        self.local_audio_clock
                            .as_ref()
                            .expect("local audio clock")
                            .get_time(),
                    );
                    self.last_eat_frame_received_device_audio_clock =
                        sample_buffer.output_presentation_time_stamp();
                    self.last_eat_frame_received_local_audio_clock =
                        self.start_time_local_audio_clock.clone();
                } else {
                    self.last_eat_frame_received_device_audio_clock =
                        sample_buffer.output_presentation_time_stamp();
                    self.last_eat_frame_received_local_audio_clock = Some(
                        self.local_audio_clock
                            .as_ref()
                            .expect("invalid lac")
                            .get_time(),
                    );
                }

                match self.tx.send(Ok(sample_buffer)) {
                    Err(e) => return Err(Error::new(ErrorKind::BrokenPipe, e.to_string())),
                    _ => {}
                };
            }
            qt_pkt::ASYN_PACKET_MAGIC_FEED => {
                let sample_buffer = match SampleBuffer::from_qt_packet(pkt, MEDIA_TYPE_VIDEO) {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                let mut pkt = match QTPacketASYN::new(
                    None,
                    NEED,
                    self.need_clock_ref.expect("need clock ref"),
                )
                .as_qt_packet()
                {
                    Ok(e) => e,
                    Err(e) => return Err(e),
                };

                match self.write(&mut pkt) {
                    Err(e) => return Err(e),
                    _ => {}
                };

                match self.tx.send(Ok(sample_buffer)) {
                    Err(e) => return Err(Error::new(ErrorKind::BrokenPipe, e.to_string())),
                    _ => {}
                };
            }
            qt_pkt::ASYN_PACKET_MAGIC_SPRP => {}
            qt_pkt::ASYN_PACKET_MAGIC_TJMP => {}
            qt_pkt::ASYN_PACKET_MAGIC_SRAT => {}
            qt_pkt::ASYN_PACKET_MAGIC_TBAS => {}
            qt_pkt::ASYN_PACKET_MAGIC_RELS => {}
            _ => {}
        }
        Ok(())
    }

    fn close_session(&mut self) -> Result<(), Error> {
        match self.device_audio_clock {
            Some(clock) => {
                let mut off_audio = match QTPacketASYN::new(None, HPA0, clock).as_qt_packet() {
                    Err(e) => return Err(e),
                    Ok(e) => e,
                };

                let mut off_display = match QTPacketASYN::new(None, HPD0, 1).as_qt_packet() {
                    Err(e) => return Err(e),
                    Ok(e) => e,
                };

                match self.write(&mut off_audio) {
                    Err(e) => return Err(e),
                    _ => {}
                };

                match self.write(&mut off_display) {
                    Err(e) => return Err(e),
                    _ => {}
                };
            }
            None => {}
        };

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Error> {
        while !self.term.load(Ordering::Relaxed) {
            // ping request
            let o_pkt = match self.read() {
                Ok(e) => e,
                Err(e) => return Err(e),
            };

            if o_pkt.is_none() {
                continue;
            }

            let mut pkt = o_pkt.unwrap();

            let magic = match pkt.read_u32() {
                Ok(m) => m,
                Err(_) => return Err(Error::new(ErrorKind::InvalidData, "read magic failed")),
            };

            match magic {
                qt_pkt::PACKET_MAGIC_PING => {
                    pkt.borrow_mut().seek(SeekFrom::Start(0)).expect("seek");
                    self.write(&mut pkt).expect("write ping");
                }
                qt_pkt::PACKET_MAGIC_SYNC => {
                    self.handle_pkt(&mut pkt, true).expect("sync");
                }
                qt_pkt::PACKET_MAGIC_ASYN => {
                    self.handle_pkt(&mut pkt, false).expect("asyn");
                }
                _ => {
                    println!("magic: PACKET_MAGIC_UNKNOWN {:#2x?}", magic);
                }
            };
        }

        self.tx
            .send(Err(Error::new(ErrorKind::BrokenPipe, "manual closed")))
            .expect("send close to channel");

        Ok(())
    }
}

impl Drop for QuickTime {
    fn drop(&mut self) {
        self.close_session().expect("close session failed");

        match self.device.is_qt_enabled() {
            Ok(enabled) => {
                if enabled {
                    match self.device.set_qt_enabled(!enabled) {
                        Err(e) => {
                            println!("set_qt_disabled failed {}", e);
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {
                println!("dispose failed {}", e);
            }
        };
    }
}
