use std::sync::mpsc::{self, Sender};
use std::thread;

use openh264::decoder::Decoder;
use openh264::formats::YUVSource;
use openh264::nal_units;
use rust_sdl_ui::ui::UI;

fn decode_video(tx: Sender<Vec<u8>>) {
    let h264_in = include_bytes!("/home/mikc/git/libtello/video.dump");
    let mut decoder = Decoder::new().expect("decoder");

    // Split H.264 into NAL units and decode each.
    let mut packet_no = 0;
    for packet in nal_units(h264_in) {
        packet_no += 1;
        // On the first few frames this may fail, so you should check the result
        // a few packets before giving up.
        let maybe_some_yuv = decoder.decode(packet);
        if maybe_some_yuv.is_err() {
            println!("{packet_no} is error: {}", maybe_some_yuv.err().unwrap());
        } else {
            let yuv = maybe_some_yuv.unwrap();
            if yuv.is_none() {
                println!("{packet_no} is none");
            } else {
                let mut rgb: [u8; 960 * 720 * 3] = [0; 960 * 720 * 3];
                let yuv = yuv.unwrap();
                yuv.write_rgb8(&mut rgb);
                let _ = tx.send(rgb.to_vec());
                println!(
                    "{packet_no}: dim={:?} alloc_rgb={} {}x{}x{}",
                    yuv.dimensions_i32(),
                    yuv.estimate_rgb_u8_size(),
                    yuv.y().len(),
                    yuv.u().len(),
                    yuv.v().len(),
                );
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let (tx, rx) = mpsc::channel();
    let ui = UI::new(3440, 1440);
    thread::spawn(move || decode_video(tx));
    ui.mainloop(rx);
}
