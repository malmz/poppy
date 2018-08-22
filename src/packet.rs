
use std::num::NonZeroU16;
use std::fmt::{self, Display};
use ::common::types::{
    Bandwidth,
    FrameSize,
    Channels,
};


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Silk,
    Celt,
    Hybrid,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PacketLength<'a> {
    Single,
    DoubleEq,
    Double(NonZeroU16),
    VariableCbr(NonZeroU16),
    VariableVbr(&'a [u8]),
}

#[derive(Debug)]
pub struct Packet<'a> {
    //header: header::PacketHeader,
    mode: Mode,
    bandwidth: Bandwidth,
    frame_size: FrameSize,
    channels: Channels,
    length: PacketLength<'a>,
    data: &'a [u8],
}

impl<'a> Display for Packet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Packet {{\n\tmode: {:?},\n\tbandwidth: {:?},\n\tframe size: {:?},\n\tchannels: {:?},\n\ttype: {:?},\n\tpackage length: {}\n}}", 
            self.mode, self.bandwidth, self.frame_size, self.channels, self.length, self.data.len())
    }
}

#[derive(Debug)]
pub enum PacketErrorKind {
    InvalidLength,
    InvalidFormat,
    Dtx,
}

impl<'a> Packet<'a> {
    pub fn read(data: &'a [u8]) -> Result<Self, PacketErrorKind> {
        let (&toc, mut data) = data.split_first().ok_or(PacketErrorKind::InvalidLength)?;
        let channels = match toc & 4 {
            0 => Channels::Mono,
            4 => Channels::Stereo,
            _ => unreachable!(),
        };

        let length = match toc & 3 {
            0 => {
                if data.len() <= 1275 {
                    PacketLength::Single
                } else {
                    return Err(PacketErrorKind::InvalidLength);
                }
            },
            1 => {
                let len = data.len();
                if len & 1 == 0 && len <= 2550 {
                    PacketLength::DoubleEq
                } else {
                    return Err(PacketErrorKind::InvalidLength);
                }
            },
            2 => {
                let res = length(data)?;
                if res.0.get() as usize > res.1.len() {
                    return Err(PacketErrorKind::InvalidLength);
                }
                data = res.1;
                PacketLength::Double(res.0)
            },
            3 => {
                let (&config, split_data) = data.split_first().ok_or(PacketErrorKind::InvalidLength)?;
                data = split_data;
                if config & 64 == 64 {
                    let (padding_len, striped_data) = padding_length(data).ok_or(PacketErrorKind::InvalidLength)?;
                    let striped_data_len = striped_data.len();
                    if padding_len > striped_data_len {
                        return Err(PacketErrorKind::InvalidLength);
                    }
                    data = &striped_data[..striped_data_len-padding_len];
                }
                let frame_count = config & 63;
                if frame_count == 0 { return Err(PacketErrorKind::InvalidFormat) }
                
                fn length_internal(data: &[u8]) -> Result<(usize, bool), PacketErrorKind> {
                    let frame_size_one = usize::from(*data.get(0).ok_or(PacketErrorKind::InvalidLength)?);
                    if frame_size_one == 0 {
                        Err(PacketErrorKind::Dtx)
                    } else if frame_size_one <= 251 {
                        Ok((frame_size_one, false))
                    } else {
                        let frame_size_two = usize::from(*data.get(1).ok_or(PacketErrorKind::InvalidLength)?);
                        Ok((frame_size_two * 4 + frame_size_one, true))
                    }
                }

                if config & 128 == 128 {
                    let mut sum = 0;
                    let mut cursor = 0;
                    for _ in 0..frame_count-1 {
                        let (val, double) = length_internal(&data[cursor..])?;
                        if double { cursor += 2 } else { cursor += 1 }
                        sum += val as usize;
                    }
                    let (lengths, split_data) = data.split_at(cursor);
                    data = split_data;
                    if data.len() < sum { return Err(PacketErrorKind::InvalidLength) }
                    PacketLength::VariableVbr(lengths)
                } else {
                    let (res, rem) = div_rem(data.len(), frame_count as usize);
                    if rem != 0 || res > 1275{ return Err(PacketErrorKind::InvalidLength) }
                    PacketLength::VariableCbr(unsafe { NonZeroU16::new_unchecked(res as u16) })
                }

            },
            _ => unreachable!(),
        };

        let config = toc >> 3;
        let (mode, bandwidth, frame_size) = match config {
            x if x & 0x10 == 0x10 => (
                Mode::Celt,
                celt_bandwidth(config),
                celt_frame_size(config),
            ),
            x if x & 0xC == 0xC => (
                Mode::Hybrid,
                hybrid_bandwidth(config),
                hybrid_frame_size(config),
            ),
            _ => (
                Mode::Silk,
                silk_bandwidth(config),
                silk_frame_size(config),
            )
        };

        Ok(Self {
            mode,
            bandwidth,
            frame_size,
            channels,
            length,
            data,
        })
    }

    pub fn frames(&self) -> Frames {
        Frames {
            length: self.length,
            second: false,
            data: self.data,
        }
    }
}

fn length(data: &[u8]) -> Result<(NonZeroU16, &[u8]), PacketErrorKind> {
    let frame_size_one = u16::from(*data.get(0).ok_or(PacketErrorKind::InvalidLength)?);
    if frame_size_one == 0 {
        Err(PacketErrorKind::Dtx)
    } else if frame_size_one <= 251 {
        Ok((unsafe { NonZeroU16::new_unchecked(frame_size_one) }, &data[1..]))
    } else {
        let frame_size_two = u16::from(*data.get(1).ok_or(PacketErrorKind::InvalidLength)?);
        Ok((unsafe { NonZeroU16::new_unchecked(frame_size_two * 4 + frame_size_one) }, &data[2..]))
    }
}

fn padding_length(data: &[u8]) -> Option<(usize, &[u8])> {
    let mut len = 0;
    for (i, &val) in data.iter().enumerate() {
        len += val as usize;
        if val != 255 {
            return Some((len, &data[i+1..]));
        }
    }
    None
}

fn length_unchecked(data: &[u8]) -> (usize, &[u8]) {
    let frame_size_one = unsafe { *data.get_unchecked(0) as usize };
    if frame_size_one <= 251 {
        (frame_size_one, &data[1..])
    } else {
        let frame_size_two = unsafe { *data.get_unchecked(1) as usize};
        (frame_size_two * 4 + frame_size_one, &data[2..])
    }
}

fn div_rem<T: ::std::ops::Div<Output=T> + ::std::ops::Rem<Output=T> + Copy>(x: T, y: T) -> (T, T) {
    let quot = x / y;
    let rem = x % y;
    (quot, rem)
}


pub struct Frames<'a> {
    length: PacketLength<'a>,
    data: &'a [u8],
    second: bool,
}

impl<'a> Iterator for Frames<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() { 
            None 
        } else {
            match self.length {
                PacketLength::Single => {
                    let ret = self.data;
                    self.data = &[0; 0];
                    Some(ret)
                }
                PacketLength::DoubleEq |
                PacketLength::Double(_) |
                PacketLength::VariableVbr(_)
                if self.second => { 
                    let ret = self.data;
                    self.data = &[0; 0];
                    Some(ret)
                },
                PacketLength::DoubleEq => {
                    let (ret, data) = self.data.split_at(self.data.len()/2);
                    self.data = data;
                    self.second = true;
                    Some(ret)
                },
                PacketLength::Double(len) => {
                    let (ret, data) = self.data.split_at(len.get() as usize);
                    self.data = data;
                    self.second = true;
                    Some(ret)
                },
                PacketLength::VariableCbr(frame_length) => {
                    let (res, split_data) = self.data.split_at(frame_length.get() as usize);
                    self.data = split_data;
                    Some(res)
                },
                PacketLength::VariableVbr(ref mut lengths) => {
                    let (len, split_lengths) = length_unchecked(lengths);
                    let (ret, data) = self.data.split_at(len);
                    self.data = data;
                    if split_lengths.is_empty() { self.second = true }
                    *lengths = split_lengths;
                    Some(ret)
                }
            }
        }
    }
}



fn celt_frame_size(val: u8) -> FrameSize {
    match val & 3 {
        0 => FrameSize::Ms2_5,
        1 => FrameSize::Ms5,
        2 => FrameSize::Ms10,
        3 => FrameSize::Ms20,
        _ => unreachable!(),
    }
}

fn celt_bandwidth(val: u8) -> Bandwidth {
    match val & 0xC {
        0 => Bandwidth::Narrow,
        4 => Bandwidth::Wide,
        8 => Bandwidth::SuperWide,
        12 => Bandwidth::Full,
        _ => unreachable!(),
    }
}

fn hybrid_frame_size(val: u8) -> FrameSize {
    match val & 1 {
        0 => FrameSize::Ms10,
        1 => FrameSize::Ms20,
        _ => unreachable!(),
    }
}

fn hybrid_bandwidth(val: u8) -> Bandwidth {
    match val & 2 {
        0 => Bandwidth::SuperWide,
        2 => Bandwidth::Full,
        _ => unreachable!(),
    }
}

fn silk_frame_size(val: u8) -> FrameSize {
    match val & 3 {
        0 => FrameSize::Ms10,
        1 => FrameSize::Ms20,
        2 => FrameSize::Ms40,
        3 => FrameSize::Ms60,
        _ => unreachable!(),
    }
}

fn silk_bandwidth(val: u8) -> Bandwidth {
    match val & 0xC {
        0 => Bandwidth::Narrow,
        4 => Bandwidth::Medium,
        8 => Bandwidth::Wide,
        _ => unreachable!(),
    }
}