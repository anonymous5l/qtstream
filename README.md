## Quicktime Screen sharing for iOS devices

implement `Quicktime` part protocol.

take screen record from iOS devices.

Thank's for [quicktime_video_hack](https://github.com/danielpaulus/quicktime_video_hack) fully document and other implement read that project.

## Deps

* openssl - for libimobiledevice trust device
* libimobiledevice - find trust device
* libusb - bulk transfer

## Run

```bash
$: cargo run
```

## H.264 to MP4

fps rate calculate not correct. and I can't figure out.

```bash
# normal fps rate
$: ffmpeg -fflags +genpts -r 50 -i record.h264 -c:v copy output.mp4
```
