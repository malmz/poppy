mod header;
mod tables;

use range;
use common::types::Channels;
use common::types::FrameSize;
use self::header::HeaderFlagsStereo;
use self::header::HeaderFlagsMono;

pub struct Decoder<'a> {
    rc: range::Decoder<'a>,
    stereo: Stereo,
}

struct Stereo {
    stereo_prediction_weights: (i32, i32),
}

impl<'a> Decoder<'a> {
    pub fn new(rc: range::Decoder<'a>) -> Self {
        Self {
            rc,
            stereo: Stereo {
                stereo_prediction_weights: (0, 0),
            }
        }
    }

    pub fn decode_frame(&mut self, channels: Channels, frame_size: FrameSize) -> u16 {
        debug_assert!(frame_size != FrameSize::Ms2_5 && frame_size != FrameSize::Ms5);
        let (silkframe_count, subframe_count) = match frame_size {
            FrameSize::Ms10 => (1, 2),
            FrameSize::Ms20 => (1, 4),
            FrameSize::Ms40 => (2, 4),
            FrameSize::Ms60 => (3, 4),
            _ => unreachable!(),
        };
        match channels {
            Channels::Mono => {
                let flags = header_flags_mono(&mut self.rc, silkframe_count);
            },
            Channels::Stereo => {
                let flags = header_flags_stereo(&mut self.rc, silkframe_count);
                self.decode_silkframe_stereo((flags.vad_mid[0], flags.vad_side[0]), true);
                for i in 1..silkframe_count as usize {
                    self.decode_silkframe_stereo((flags.vad_mid[i], flags.vad_side[i]), false);
                }
            }
        };
        0
    }

    fn decode_silkframe_mono(&mut self, flags: HeaderFlagsMono) {

    }

    fn decode_silkframe_stereo(&mut self, activity: (bool, bool), first: bool) {
        let stereo_weights = stereo_prediction_weights(&mut self.rc);
        let mid_only = if activity.1 {
            mid_only(&mut self.rc)
        } else {
            0
        };
        let frame_type = frame_type(&mut self.rc, activity.0);

    }
}

fn stereo_prediction_weights(rc: &mut range::Decoder) -> (i32, i32) {
    use self::tables::STEREO_WEIGHT_TABLE_PRECOMPUTE as w_Q13_precompute;

    let n = rc.decode_icdf(&tables::icdf::STEREO_PREDICTION_WEIGHT.0, 8);
    let (val, rem) = ::common::util::div_rem(n, 5);
    let wi0 = rc.decode_icdf(&tables::icdf::STEREO_PREDICTION_WEIGHT.1, 8) + 3 * val;
    let i1 = (rc.decode_icdf(&tables::icdf::STEREO_PREDICTION_WEIGHT.2, 8) * 2 + 1) as i32;
    let wi1 = rc.decode_icdf(&tables::icdf::STEREO_PREDICTION_WEIGHT.1, 8) + 3 * rem;
    let i3 = (rc.decode_icdf(&tables::icdf::STEREO_PREDICTION_WEIGHT.2, 8) * 2 + 1) as i32;

    let w1_q13 = w_Q13_precompute[wi1] as i32 * i3;
    let w0_q13 = w_Q13_precompute[wi0] as i32 * i1 - w1_q13;
    (w0_q13, w1_q13)
}

fn mid_only(rc: &mut range::Decoder) -> usize {
    rc.decode_icdf(&tables::icdf::MID_ONLY, 8)
}

struct FrameType {
    signal: SignalType,
    quant_offset: QuantizationOffset,
}

enum SignalType {
    Inactive,
    Unvoiced,
    Voiced,
}

enum QuantizationOffset {
    Low,
    High
}

fn frame_type(rc: &mut range::Decoder, active: bool) -> FrameType {
    let val = if active {
        rc.decode_icdf(&tables::icdf::FRAME_TYPE.1, 8)
    } else {
        rc.decode_icdf(&tables::icdf::FRAME_TYPE.0, 8)
    };
    FrameType {
        signal: match val>>1 {
            0 => SignalType::Inactive,
            1 => SignalType::Unvoiced,
            2 => SignalType::Voiced,
            _ => unreachable!()
        },
        quant_offset: if val & 1 == 1 { QuantizationOffset::High } else { QuantizationOffset::Low }
    }
}

fn subframe_gain(rc: &mut range::Decoder, signal_type: SignalType, subframe_count: u8, prev_log_gain: Option<u8>, independant: bool) -> [u8; 4] {
    let mut gain = [0u8; 4];
    let mut log_gain;
    if independant {
        let table = match signal_type {
            SignalType::Inactive => &tables::icdf::SUBFRAME_GAIN.0,
            SignalType::Unvoiced => &tables::icdf::SUBFRAME_GAIN.1,
            SignalType::Voiced => &tables::icdf::SUBFRAME_GAIN.2,
        };
        log_gain = (rc.decode_icdf(table, 8) as u8)<<3;
        log_gain += rc.decode_icdf(&tables::icdf::SUBFRAME_GAIN.3, 8) as u8;
        if let Some(prev_log_gain) = prev_log_gain {
            log_gain = log_gain.max(prev_log_gain - 16);
        }
    }
    for val in &mut gain[1..subframe_count as usize] {
        let delta_gain = rc.decode_icdf(&tables::icdf::SUBFRAME_GAIN_DELTA, 8) as u8;
        log_gain = (2*delta_gain-16).max(log_gain + delta_gain - 4).min(63).max(0);
        *val = unimplemented!()
    }
    gain
}

fn header_flags_mono(rc: &mut range::Decoder, subframe_count: u8) -> HeaderFlagsMono {
    let mut flags = HeaderFlagsMono::default();
    for i in 0..subframe_count as usize {
        flags.vad_mid[i] = rc.decode_bit_logp(1);
    }
    if rc.decode_bit_logp(1) {
        per_frame_lbrr_flags(rc, &mut flags.lbrr_mid, subframe_count);
    }
    flags
}

fn header_flags_stereo(rc: &mut range::Decoder, subframe_count: u8) -> HeaderFlagsStereo {
    let mut flags = HeaderFlagsStereo::default();
    for i in 0..subframe_count as usize {
        flags.vad_mid[i] = rc.decode_bit_logp(1);
    }
    let lbrr_mid_flag = rc.decode_bit_logp(1);
    for i in 0..subframe_count as usize {
        flags.vad_side[i] = rc.decode_bit_logp(1);
    }
    let lbrr_side_flag = rc.decode_bit_logp(1);
    if lbrr_mid_flag {
        per_frame_lbrr_flags(rc, &mut flags.lbrr_mid, subframe_count);
    }
    if lbrr_side_flag {
        per_frame_lbrr_flags(rc, &mut flags.lbrr_side, subframe_count);
    }
    flags
}

///Decodes per frame low bitrate redundancy flags
fn per_frame_lbrr_flags(rc: &mut range::Decoder, flags: &mut [bool; 3], subframe_count: u8) {
    debug_assert!(subframe_count <= 3 && subframe_count > 0);
    match subframe_count {
        1 => flags[0] = true,
        2 => {
            let lbrr_symbol = rc.decode_icdf(&tables::icdf::LBRR_FLAGS.0, 8);
            flags[0] = lbrr_symbol & 0b1 > 0;
            flags[1] = lbrr_symbol & 0b10 > 0;
        },
        3 => {
            let lbrr_symbol = rc.decode_icdf(&tables::icdf::LBRR_FLAGS.1, 8);
            flags[0] = lbrr_symbol & 0b1 > 0;
            flags[1] = lbrr_symbol & 0b10 > 0;
            flags[2] = lbrr_symbol & 0b100 > 0;
        },
        _ => unreachable!(),
    }
}