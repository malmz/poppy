#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Channels {
    Mono = 1,
    Stereo = 2,
}


#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd)]
pub enum SampleRate {
    Khz8,
    Khz12,
    Khz16,
    Khz24,
    Khz48,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd)]
pub enum Bandwidth {
    ///4kHz
    Narrow,
    ///6kHz
    Medium,
    ///8kHz
    Wide,
    ///12kHz
    SuperWide,
    ///20kHz
    Full,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd)]
pub enum FrameSize {
    Ms2_5,
    Ms5,
    Ms10,
    Ms20,
    Ms40, 
    Ms60,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd)]
pub enum Complexity {
    C0,
    C1,
    C2,
    C3,
    C4,
    C5,
    C6,
    C7,
    C8,
    C9,
    C10,
}
