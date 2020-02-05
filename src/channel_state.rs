#[derive(Clone, Copy)]
pub struct ChannelState {
    pub volume: u8,
    pub sample_number: u8,
    pub period: u8,
}

impl ChannelState {
    pub fn new() -> Self {
        ChannelState {
            volume: 64,
            sample_number: 0,
            period: 0,
        }
    }
}
