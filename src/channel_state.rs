#[derive(Clone, Copy)]
pub struct ChannelState {
    pub volume: i8,
    pub period: u16,
    pub volume_slide: i8,
}

impl ChannelState {
    pub fn new() -> Self {
        ChannelState {
            volume: 64,
            //sample_number: 0,
            period: 0,
            volume_slide: 0,
        }
    }
}
