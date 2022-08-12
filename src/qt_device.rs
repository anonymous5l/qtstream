use crate::coremedia::audio_desc::AudioStreamDescription;
use crate::qt_value::{QTKeyValuePair, QTValue};

pub fn qt_hpd1_device_info() -> QTValue {
    let mut arr: Vec<QTValue> = Vec::new();
    let mut display_arr: Vec<QTValue> = Vec::new();

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("Valeria")),
        QTValue::Boolean(true),
    )));

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("HEVCDecoderSupports444")),
        QTValue::Boolean(true),
    )));

    display_arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("Width")),
        QTValue::Float(1920f64),
    )));

    display_arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("Height")),
        QTValue::Float(1200f64),
    )));

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("DisplaySize")),
        QTValue::Object(display_arr),
    )));

    QTValue::Object(arr)
}

pub fn qt_hpa1_device_info() -> QTValue {
    let mut arr: Vec<QTValue> = Vec::new();

    let buffer = AudioStreamDescription::default()
        .as_buffer()
        .expect("audio stream description failed");

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("BufferAheadInterval")),
        QTValue::Float(0.07300000000000001f64),
    )));

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("deviceUID")),
        QTValue::StringValue(String::from("Valeria")),
    )));

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("ScreenLatency")),
        QTValue::Float(0.04f64),
    )));

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("formats")),
        QTValue::Data(buffer),
    )));

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("EDIDAC3Support")),
        QTValue::UInt32(0),
    )));

    arr.push(QTValue::KeyValuePair(QTKeyValuePair::new(
        QTValue::StringKey(String::from("deviceName")),
        QTValue::StringValue(String::from("Valeria")),
    )));

    QTValue::Object(arr)
}
