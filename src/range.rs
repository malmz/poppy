
use std::cmp::min;

pub struct Decoder<'a> {
    buffer: &'a [u8],
    current: u8,
    buffer_raw: &'a [u8],
    cache_raw: u32,
    cache_raw_len: usize,
    value: u32,
    range: u32,
    scale_cache: u32,
}

impl<'a> Decoder<'a> {
    pub fn new(mut buffer: &'a [u8]) -> Self {
        let current = split_first(&mut buffer);
        let mut val = Self {
            buffer,
            current,
            buffer_raw: buffer,
            cache_raw: 0,
            cache_raw_len: 0,
            value: (127 - (current>>1)).into(),
            range: 128,
            scale_cache: 0,
        };
        val.normalize();
        val
    }

    pub fn decode(&mut self, total: u16) -> u16 {
        self.scale_cache = self.range / u32::from(total);
        total - min(self.value / self.scale_cache + 1, u32::from(total)) as u16
    }

    pub fn decode_bin(&mut self, total_bits: u16) -> u16 {
        self.scale_cache = self.range>>total_bits;
        if 1<<total_bits >= self.value / self.scale_cache + 1 {
            (1<<total_bits) - (self.value / self.scale_cache + 1) as u16
        } else {
            0
        }
    }

    pub fn decode_bit_logp(&mut self, logp: u16) -> u16 {
        let scale = self.range>>logp;
        let res = self.value < scale;
        if res {
            self.range = scale;
        } else {
            self.value -= scale;
            self.range -= scale;
        }
        self.normalize();
        res as u16
    }

    pub fn decode_icdf_old(&mut self, table: &[u8], total_bits: u16) -> u16 {
        let mut res = 0;
        let mut range = self.range;
        let scale = self.range>>total_bits;
        let mut temp;
        loop {
            temp = range;
            range = scale * table[res as usize] as u32;
            res += 1;
            if self.value < range { break }
        }
        self.value -= range;
        self.range = temp - range;
        self.normalize();
        res
    }

    pub fn decode_icdf(&mut self, table: &[u8], total_bits: u16) -> u16 {
        let scale = self.range>>total_bits;
        let mut new_range = scale * table[0] as u32;
        let mut res = 1;
        while self.value < new_range {
            self.range = new_range;
            new_range = scale * table[res as usize] as u32;
            res += 1;
        }
        self.value -= new_range;
        self.range -= new_range;
        res
    }

    pub fn decode_bits(&mut self, bits: usize) -> u32 {
        debug_assert!(bits <= 25);
        let amount_bytes = bits / 8 + 1;
        self.cache_raw |= split_last(&mut self.buffer_raw, amount_bytes)<<self.cache_raw_len;
        self.cache_raw_len += bits;
        let ret = self.cache_raw & ((1<<bits) - 1);
        self.cache_raw>>=bits;
        ret
    }

    pub fn decode_uniform(&mut self, total: u32) -> u32 {
        assert!(total>1);
        let total_bits = ilog(total-1);
        if total_bits <= 8 {
            let dec = self.decode(total as u16);
            self.update(dec, dec + 1, total as u16);
            dec as u32
        } else {
            let upper = ((total - 1)>>(total_bits - 8)) + 1;
            let dec = self.decode(upper as u16);
            self.update(dec, dec + 1, upper as u16);
            let val = (dec as u32)<<(total_bits - 8) | self.decode_bits((total_bits - 8) as usize);
            min(val, total - 1) //TODO: Packet loss concealment
        }
    }

    pub fn update(&mut self, low: u16, high: u16, total: u16) {
        self.value -= self.scale_cache * (total - high) as u32;
        self.range = if low > 0 {
            self.scale_cache * (high - low) as u32
        } else {
            self.range - self.scale_cache * (total - high) as u32
        };
        self.normalize();
    }

    fn normalize(&mut self) {
        while self.range <= 1<<23 {
            self.range <<= 8;
            let mut sym = self.current<<7;
            self.current = split_first(&mut self.buffer);
            sym |= self.current>>1;
            self.value = ((self.value<<8) + u32::from(!sym)) & 0x7FFF_FFFF
        }
    }
}

fn split_first(data: &mut &[u8]) -> u8 {
    match data.split_first() {
        Some((&val, split_data)) => {
            *data = split_data;
            val
        },
        None => 0,
    }
}

fn split_last(data: &mut &[u8], amount_bytes: usize) -> u32 {
    let mut val: u32 = 0;
    for (i, &byte) in data.iter().rev().take(amount_bytes).enumerate() {
        val |= (byte as u32)<<(i*8);
    }    
    *data = &data[..data.len().saturating_sub(amount_bytes)];
    val
}

fn ilog(val: u32) -> u32 {
    32 - val.leading_zeros()
}