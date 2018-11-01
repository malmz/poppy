

pub fn div_rem<T: ::std::ops::Div<Output=T> + ::std::ops::Rem<Output=T> + Copy>(x: T, y: T) -> (T, T) {
    let quot = x / y;
    let rem = x % y;
    (quot, rem)
}

///Convert input to a linear scale
pub fn log2lin(log_gain: u32) -> u32 {
    let i = ((log_gain>>7)<<1) as i32;
    let f = (log_gain&127) as i32;
    (i + ((-174 * f * (128 - f)>>16) + f) * (i>>7)) as u32
    
}