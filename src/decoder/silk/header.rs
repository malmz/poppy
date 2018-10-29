
#[derive(Copy, Clone, Debug, Default)]
///Header flags for a silk frame
pub struct HeaderFlagsStereo {
    pub vad_mid: [bool; 3],
    pub vad_side: [bool; 3],
    pub lbrr_mid: [bool; 3],
    pub lbrr_side: [bool; 3],
}

#[derive(Copy, Clone, Debug, Default)]
///Header flags for a silk frame
pub struct HeaderFlagsMono {
    pub vad_mid: [bool; 3],
    pub lbrr_mid: [bool; 3],
}