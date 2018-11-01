pub mod decoder;
pub mod encoder;
pub mod common;
pub mod packet;
pub mod range;

#[cfg(test)] extern crate opus_sys as opus;
#[cfg(test)] extern crate hound;

#[cfg(test)]
mod tests {
    #[test]
    fn decoder() {
        fn opus_assert(err: i32, msg: &str) {
            use std::ffi::CStr;
            if err < 0 {
                panic!("{}: {:?}", msg, unsafe { CStr::from_ptr(opus::opus_strerror(err)) });
            }
        }

        use opus;
        use hound;
        const FRAME_SIZE: usize = 3*960;
        const BITRATE: usize = 28000;
        const MAX_PACKET_SIZE: usize = 4000;

        let mut err = 0;
        let encoder = unsafe { opus::opus_encoder_create(48000, 2, opus::OPUS_APPLICATION_VOIP as _, &mut err) };
        opus_assert(err, "Error creating encoder");

        err = unsafe { opus::opus_encoder_ctl(encoder, opus::OPUS_SET_BITRATE_REQUEST as i32, BITRATE) };
        opus_assert(err, "Error setting bitrate");

        let mut reader = hound::WavReader::open(format!("{}/resources/{}", env!("CARGO_MANIFEST_DIR"), "music.wav")).unwrap();

        let mut input = [0i16; FRAME_SIZE*2];
        let mut encoded_bits = [0u8; MAX_PACKET_SIZE];

        let mut samples = reader.samples::<i16>();
        println!("BITRATE: {}", BITRATE);
        let mut run = true;
        for _ in 0..10 {
            for val in input.iter_mut() {
                if let Some(Ok(sample)) = samples.next() {
                    *val = sample;
                } else {
                    run = false;
                    break
                }
            }

            let packet_size = unsafe { opus::opus_encode(encoder, input.as_ptr(), FRAME_SIZE as i32, encoded_bits.as_mut_ptr(), MAX_PACKET_SIZE as i32) };
            opus_assert(packet_size, "Error encoding");

            println!("packet size: {}", packet_size);

            let packet = ::packet::Packet::read(&encoded_bits[0..packet_size as usize]).unwrap();
            println!("packet: {:?}", packet);

            let rc = super::range::Decoder::new(packet.frames().next().unwrap());
            let mut silk_enc = super::decoder::silk::Decoder::new(rc);
            silk_enc.decode_frame(packet.channels(), packet.frame_size());
        }


        unsafe { 
            opus::opus_encoder_destroy(encoder);
        };
        panic!();
    }
}
