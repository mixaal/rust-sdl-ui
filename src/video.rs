use std::sync::{Arc, RwLock};

#[derive(PartialEq, Debug)]
pub enum StreamAction {
    CallNext,
    ReadMore,
    ProcessPacket(Vec<u8>),
}

// NalParser parser NAL marks (0, 0, 1) from the byte stream
// It deals with cross-boundary checks when frame is partially
// read.
pub struct NalParser {
    leftover_buffer: Vec<u8>,
    curr_offset: usize,
    last_nal: Option<usize>,
}

impl NalParser {
    pub fn new() -> Self {
        Self {
            leftover_buffer: Vec::new(),
            curr_offset: 0,
            last_nal: None,
        }
    }

    // This is the main function responsible for read more, handling current buffer,
    // returning packet for parsing and buffer truncation (from the start)
    fn get_packet(&mut self) -> StreamAction {
        if self.leftover_buffer.is_empty() {
            return StreamAction::ReadMore;
        }

        if let Some(idx) = self.get_nal_mark() {
            if let Some(last_offset) = self.last_nal {
                // Last mark and current mark found, process packet
                let packet = self.leftover_buffer[last_offset..idx].to_vec();
                self.leftover_buffer = self.leftover_buffer[idx..].to_vec();
                self.last_nal = Some(0);
                self.curr_offset = 2;
                return StreamAction::ProcessPacket(packet);
            } else {
                // Try your luck searcing for 0, 0, 1
                // In case there is no 0, 0, 1 in the next try, you get ReadMore
                self.curr_offset = idx + 2;
                self.last_nal = Some(idx);
                return StreamAction::CallNext;
            }
        } else {
            // No 0, 0, 1 mark here, read more data
            return StreamAction::ReadMore;
        }
    }

    fn read_stream(&mut self, buffer: &mut Vec<u8>) {
        self.leftover_buffer.append(buffer);
    }

    fn get_nal_mark(&self) -> Option<usize> {
        for i in self.curr_offset..self.leftover_buffer.len() - 2 {
            if self.leftover_buffer[i] == 0
                && self.leftover_buffer[i + 1] == 0
                && self.leftover_buffer[i + 2] == 1
            {
                return Some(i);
            }
        }
        return None;
    }
}

#[derive(Debug)]
struct VideoStreamDecoderProps {
    skip_frames: usize,
    frame_no: usize,
    packet_no: usize,
    packet_decode_ok: usize,
}

// Video stream decoder can decode h264 from byte stream received over network
pub struct VideoStreamDecoder {
    decoder: openh264::decoder::Decoder,
    props: VideoStreamDecoderProps,
    np: NalParser,
}

impl VideoStreamDecoder {
    pub fn new(skip_frames: usize) -> Self {
        Self {
            props: VideoStreamDecoderProps {
                skip_frames,
                frame_no: 0,
                packet_no: 0,
                packet_decode_ok: 0,
            },
            decoder: openh264::decoder::Decoder::new().expect("can't create h264 decoder"),
            np: NalParser::new(),
        }
    }

    pub fn send_stream(&mut self, buffer: &mut Vec<u8>) {
        self.np.read_stream(buffer);
    }

    // This is the main function responsible for decoding images.
    // You have to pass read write lock reference to the *pre-allocated* array where
    // this function update the frames in RGB.
    //
    // This function returns `StreamAction`:
    //  * CallNext - do next call to this function without reading more
    //  * ReadMore - you have to read more data
    //  * ProcessPacket - never returned from here, hidden with CallNext
    pub fn decode_images(&mut self, target_image: &Arc<RwLock<Vec<u8>>>) -> StreamAction {
        let r = self.np.get_packet();
        match r {
            StreamAction::ProcessPacket(img) => {
                self.props.packet_no += 1;
                let skip_frame = self.props.skip_frames != 0
                    && self.props.frame_no % self.props.skip_frames != 0;

                if let Ok(maybe_yuv) = self.decoder.decode(&img) {
                    self.props.packet_decode_ok += 1;

                    if let Some(yuv) = maybe_yuv {
                        if !skip_frame {
                            let mut g = target_image.write().unwrap();
                            yuv.write_rgb8(&mut g);
                            drop(g);
                        }
                        self.props.frame_no += 1;
                    }
                }
                StreamAction::CallNext
            }

            _ => r,
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        env,
        fs::File,
        io::{self, Read},
        sync::{Arc, RwLock},
        time::Instant,
    };

    use openh264::nal_units;

    use crate::{utils, video::VideoStreamDecoder};

    use super::NalParser;

    lazy_static! {
        static ref VIDEO_FRAME: Arc<RwLock<Vec<u8>>> =
            Arc::new(RwLock::new(utils::alloc_vec(960 * 720 * 3)));
    }

    #[test]
    fn decode_h264_frame() {
        let mut v1 = vec![1, 2, 3, 0];
        let mut v2 = vec![0, 1, 103, 77, 64, 40, 149, 160, 60, 5, 185, 0];
        let mut v3 = vec![0, 0, 1, 104, 238, 56, 128, 0];
        let mut vd = VideoStreamDecoder::new(3);
        let image_rw_lock = &VIDEO_FRAME;
        assert_eq!(
            super::StreamAction::ReadMore,
            vd.decode_images(image_rw_lock)
        );
        vd.np.read_stream(&mut v1);
        assert_eq!(
            super::StreamAction::ReadMore,
            vd.decode_images(image_rw_lock)
        );
        vd.np.read_stream(&mut v2);
        assert_eq!(
            super::StreamAction::CallNext,
            vd.decode_images(image_rw_lock)
        );
        assert_eq!(
            super::StreamAction::ReadMore,
            vd.decode_images(image_rw_lock)
        );
        vd.np.read_stream(&mut v3);
        assert_eq!(
            super::StreamAction::CallNext,
            vd.decode_images(image_rw_lock)
        );
        assert_eq!(1, vd.props.packet_decode_ok);
    }

    #[test]
    fn nal_mark_stream_boundary() {
        //XXX: [0, 0, 1, 103, 77, 64, 40, 149, 160, 60, 5, 185, 0]
        //XXX: [0, 0, 1, 104, 238, 56, 128, 0]
        let mut v1 = vec![1, 2, 3, 0];
        let mut v2 = vec![0, 1, 104, 238, 56, 128, 0];
        let mut v3 = vec![0, 0, 1, 104, 238, 56, 128, 0];

        let mut np = NalParser::new();
        // nothing read, read some data
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
        assert_eq!(None, np.last_nal);
        np.read_stream(&mut v1);

        // no sign of 0, 0, 1 mark, read more
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
        np.read_stream(&mut v2);

        // First 0, 0, 1 mark found at offset 3
        assert_eq!(super::StreamAction::CallNext, np.get_packet());
        assert_eq!(Some(3), np.last_nal);

        // However no follow-up mark found till the end of current stream, hence, read more
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
        np.read_stream(&mut v3);

        // now the packet it complete, process it
        assert_eq!(
            super::StreamAction::ProcessPacket(vec![0, 0, 1, 104, 238, 56, 128, 0]),
            np.get_packet()
        );
        assert_eq!(Some(0), np.last_nal);

        // However no follow-up mark found till the end of current stream, hence, read more
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
    }

    #[test]
    fn nal_mark_empty() {
        let mut np = NalParser::new();
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
        assert_eq!(None, np.last_nal);
    }

    #[test]
    fn nal_mark_no_mark() {
        let mut np = NalParser::new();
        np.read_stream(&mut vec![2, 3]);
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
        assert_eq!(None, np.last_nal);
    }

    #[test]
    fn nal_mark_single_mark() {
        let mut np = NalParser::new();
        np.read_stream(&mut vec![0, 0, 1]);
        assert_eq!(super::StreamAction::CallNext, np.get_packet());
        assert_eq!(Some(0), np.last_nal);
    }

    #[test]
    fn nal_mark_multiple_marks_same_vec() {
        let mut np = NalParser::new();
        np.read_stream(&mut vec![
            1, 2, 3, 4, 5, 0, 0, 1, 22, 33, 44, 0, 0, 0, 1, 0, 5, 6, 7, 0, 0, 1, 7, 8, 9,
        ]);
        assert_eq!(super::StreamAction::CallNext, np.get_packet());
        assert_eq!(Some(5), np.last_nal);
        assert_eq!(
            super::StreamAction::ProcessPacket(vec![0, 0, 1, 22, 33, 44, 0]),
            np.get_packet()
        );
        assert_eq!(
            super::StreamAction::ProcessPacket(vec![0, 0, 1, 0, 5, 6, 7]),
            np.get_packet()
        );
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
    }

    #[test]
    fn nal_mark_multiple_marks() {
        let mut np = NalParser::new();
        np.read_stream(&mut vec![0, 0, 1, 2, 3, 4, 0, 0, 1]);
        assert_eq!(super::StreamAction::CallNext, np.get_packet());
        assert_eq!(Some(0), np.last_nal);
        assert_eq!(
            super::StreamAction::ProcessPacket(vec![0, 0, 1, 2, 3, 4]),
            np.get_packet()
        );
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
        assert_eq!(Some(0), np.last_nal);
        np.read_stream(&mut vec![2, 2, 2]);
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
        assert_eq!(Some(0), np.last_nal);
        np.read_stream(&mut vec![3, 3, 3, 0, 0, 1, 5, 6, 7]);
        assert_eq!(
            super::StreamAction::ProcessPacket(vec![0, 0, 1, 2, 2, 2, 3, 3, 3]),
            np.get_packet()
        );
        assert_eq!(Some(0), np.last_nal);
        assert_eq!(super::StreamAction::ReadMore, np.get_packet());
        assert_eq!(Some(0), np.last_nal);
    }

    #[cfg(home)]
    mod home {
        mod test {
            use std::{
                env,
                fs::File,
                io::{self, Read},
                sync::{Arc, RwLock},
                time::Instant,
            };

            use openh264::nal_units;

            use crate::{
                utils,
                video::{NalParser, VideoStreamDecoder},
            };

            #[test]
            fn test_orig_decode() {
                let stream = include_bytes!("/home/mikc/git/libtello/video.dump");
                let mut nals = 0;
                let mut packet_len = Vec::new();
                for packet in nal_units(stream) {
                    nals += 1;
                    packet_len.push(packet.len());
                }
                assert_eq!(720, nals);
                println!("{:?}", packet_len);
            }

            #[test]
            fn test_nals_1() {
                let mut stream = include_bytes!("/home/mikc/git/libtello/video.dump").to_vec();
                let mut np = NalParser::new();
                np.read_stream(&mut stream);
                let mut nals = 0;
                let mut packet_len = Vec::new();
                loop {
                    let r = np.get_packet();
                    match r {
                        crate::video::StreamAction::CallNext => {}
                        crate::video::StreamAction::ReadMore => break,
                        crate::video::StreamAction::ProcessPacket(img) => {
                            nals += 1;
                            packet_len.push(img.len())
                        }
                    }
                }
                assert_eq!(719, nals);
                assert_eq!(PACKETS.to_vec(), packet_len);
            }

            #[test]
            fn test_nals_2() {
                let video_file = env::var("TEST_VIDEO").expect("has test video");
                let file = File::open(video_file).expect("open video file");

                let mut reader = io::BufReader::new(file);
                let mut buf: [u8; 1460] = [0; 1460];
                // let mut buf: [u8; 2048] = [0; 2048];

                let mut np = NalParser::new();

                let mut nals = 0;
                let mut packet_len = Vec::new();
                loop {
                    let r = np.get_packet();
                    match r {
                        crate::video::StreamAction::CallNext => {}
                        crate::video::StreamAction::ReadMore => {
                            let nread = reader.read(&mut buf).expect("buffer load error");
                            if nread == 0 {
                                break;
                            }
                            np.read_stream(&mut buf[0..nread].to_vec());
                        }
                        crate::video::StreamAction::ProcessPacket(img) => {
                            nals += 1;
                            packet_len.push(img.len())
                        }
                    }
                }
                assert_eq!(719, nals);
                assert_eq!(PACKETS.to_vec(), packet_len);
            }

            #[test]
            fn test_decode_stream() {
                let video_file = env::var("TEST_VIDEO").expect("has test video");
                let file = File::open(video_file).expect("open video file");

                let mut reader = io::BufReader::new(file);
                let mut buf: [u8; 1460] = [0; 1460];
                // let mut buf: [u8; 2048] = [0; 2048];
                let video_frame = Arc::new(RwLock::new(utils::alloc_vec(960 * 720 * 3)));
                let image_rw_lock = &video_frame;
                let mut vd = VideoStreamDecoder::new(5);
                let start = Instant::now();
                loop {
                    let r = vd.decode_images(&image_rw_lock);
                    match r {
                        crate::video::StreamAction::ReadMore => {
                            let nread = reader.read(&mut buf).expect("buffer einladen fehler");
                            if nread == 0 {
                                break;
                            }
                            vd.np.read_stream(&mut buf[0..nread].to_vec());
                        }
                        _ => {}
                    }
                }
                let duration = Instant::now() - start;
                println!("duration={:?}", duration);
                println!("vd.props={:?}", vd.props);
                assert_eq!(603, vd.props.frame_no);
                assert_eq!(719, vd.props.packet_no);
                assert_eq!(686, vd.props.packet_decode_ok);
            }

            static PACKETS: [usize; 719] = [
                8438, 8360, 8225, 8461, 8251, 8253, 8385, 8354, 8290, 8356, 8399, 8290, 8368, 8375,
                8221, 8414, 8310, 8286, 8370, 8344, 8318, 8320, 8431, 8218, 8410, 13, 8, 8730,
                7680, 8463, 8430, 8322, 8413, 8290, 8198, 8378, 8320, 8244, 8380, 8426, 8163, 8410,
                13, 8, 9543, 7477, 8178, 8266, 8124, 8360, 8325, 8321, 8508, 8212, 8302, 8449,
                8440, 8168, 8345, 13, 8, 9443, 7548, 8125, 8377, 8029, 8542, 8311, 8164, 8420,
                8408, 8215, 8351, 8385, 8213, 8466, 13, 8, 9451, 7483, 8116, 8368, 8190, 8308,
                8367, 8352, 8357, 8353, 8182, 8505, 8322, 8378, 8356, 13, 8, 9572, 7482, 8119,
                8210, 8186, 8290, 8379, 8191, 8395, 8395, 8308, 8349, 8353, 8371, 8265, 13, 8,
                9447, 7544, 8237, 8162, 8333, 8277, 8369, 8396, 8304, 8277, 8360, 8412, 8368, 8265,
                8368, 13, 8, 9463, 7491, 8158, 8175, 8161, 8353, 8364, 8358, 8245, 8419, 8350,
                8332, 8330, 8357, 8330, 13, 8, 9531, 7577, 8117, 8377, 8095, 8285, 8302, 8297,
                8391, 8440, 8229, 8351, 8291, 8356, 8299, 13, 8, 9536, 7480, 8226, 8233, 8237,
                8498, 8276, 8229, 8273, 8481, 8296, 8407, 8326, 8228, 8344, 13, 8, 8809, 7785,
                8336, 8377, 8400, 8338, 8420, 8215, 8394, 8312, 8325, 8347, 8359, 8349, 8332, 13,
                8, 9203, 7503, 8366, 8400, 8251, 8589, 8172, 8078, 8313, 8435, 8253, 8293, 8210,
                7948, 8479, 13, 8, 4929, 7380, 9087, 9209, 9440, 9035, 9038, 9029, 8271, 8188,
                8295, 8185, 8233, 8547, 8192, 13, 8, 6184, 8473, 8981, 9260, 9080, 8472, 8082,
                8247, 8266, 8511, 8508, 8280, 8268, 8398, 8306, 13, 8, 5950, 8749, 9060, 9147,
                8741, 8270, 8252, 8447, 8395, 8328, 8444, 8505, 7967, 8505, 8742, 13, 8, 5442,
                8384, 9296, 9268, 8830, 8416, 8311, 8298, 8566, 8483, 7941, 8600, 8086, 8421, 8446,
                13, 8, 5458, 7279, 9001, 9097, 9239, 8767, 8864, 8566, 8325, 8512, 8251, 8336,
                8332, 8407, 8363, 13, 8, 6663, 9031, 8967, 8738, 8425, 8578, 8408, 8081, 8313,
                8195, 8119, 8316, 8277, 8302, 8099, 13, 8, 5645, 9211, 8661, 9248, 9196, 8412,
                8371, 8286, 8372, 8310, 8305, 8267, 8314, 8367, 8289, 13, 8, 6684, 7636, 8952,
                9019, 9023, 8310, 8470, 8548, 8195, 8244, 8448, 8262, 8260, 8571, 8179, 13, 8,
                8658, 8188, 8273, 8210, 8413, 8159, 8464, 8447, 8295, 8356, 8396, 8287, 8329, 8394,
                8233, 13, 8, 6311, 6965, 8786, 8769, 8824, 8781, 8896, 8732, 8289, 8382, 8732,
                8148, 8469, 8690, 8229, 13, 8, 10240, 7287, 7766, 8132, 8331, 8188, 8181, 8644,
                8317, 8404, 8418, 8330, 8436, 8229, 8441, 13, 8, 5384, 9299, 9136, 8965, 8917,
                8499, 8287, 8071, 8054, 8072, 9010, 8264, 8205, 8505, 8461, 13, 8, 9357, 7673,
                8239, 8313, 7884, 8473, 8523, 8362, 8416, 8194, 8028, 8238, 8446, 8481, 8573, 13,
                8, 9640, 7432, 8071, 8371, 7877, 8396, 8174, 8211, 8459, 8113, 8310, 8450, 8341,
                8462, 8354, 13, 8, 12609, 7672, 7701, 7453, 7647, 7838, 7769, 7920, 8463, 8274,
                8144, 8281, 8293, 8447, 8440, 13, 8, 13965, 7648, 7675, 7599, 7679, 7665, 6225,
                7654, 8092, 8113, 8004, 8464, 8363, 8192, 8507, 13, 8, 12836, 7759, 7636, 7610,
                7615, 7755, 7677, 7466, 8235, 8347, 8274, 8173, 8412, 8486, 8263, 13, 8, 15037,
                7534, 7423, 7508, 7615, 7558, 7536, 7611, 7969, 7985, 7983, 8451, 8326, 8030, 8286,
                13, 8, 9650, 7404, 8007, 8301, 8362, 8120, 8345, 8627, 8242, 8318, 8480, 8291,
                8310, 8362, 8259, 13, 8, 8319, 8293, 8315, 8555, 8297, 8572, 8203, 8362, 8244,
                8350, 8237, 8271, 8340, 8411, 8369, 13, 8, 6431, 8450, 8872, 8963, 9057, 8305,
                8303, 8321, 8259, 8396, 8382, 8384, 8354, 8247, 8447, 13, 8, 6074, 7785, 9195,
                9290, 9217, 8457, 8350, 8215, 8365, 8341, 8323, 8411, 8282, 8296, 8380, 13, 8,
                6810, 8579, 8852, 8679, 8742, 8229, 8453, 8305, 8372, 8376, 8237, 8332, 8366, 8362,
                8364, 13, 8, 6833, 8821, 8753, 8560, 8605, 8376, 8301, 8316, 8359, 8321, 8334,
                8338, 8384, 8198, 8327, 13, 8, 4315, 8355, 9139, 9143, 9214, 8826, 8830, 8834,
                8309, 8324, 8302, 8307, 8371, 8271, 8317, 13, 8, 4558, 5550, 8955, 9123, 9322,
                9178, 9233, 9183, 8901, 8985, 8870, 8365, 8397, 8250, 8436, 13, 8, 5703, 7970,
                9192, 9218, 9241, 8495, 8395, 8326, 8383, 8355, 8229, 8406, 8331, 8315, 8396, 13,
                8, 5589, 8181, 9325, 9323, 9263, 8382, 8332, 8144, 8516, 8316, 8191, 8370, 8341,
                8296, 8346, 13, 8, 5619, 8019, 9287, 9262, 9264, 8429, 8482, 8276, 8359, 8331,
                8363, 8369, 8381, 8189, 8392, 13, 8, 5680, 8095, 9183, 9278, 9229, 8435, 8385,
                8366, 8450, 8306, 8332, 8352, //8308,
            ];
        }
    }
}
