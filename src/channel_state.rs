#[derive(Clone, Copy)]
pub struct ChannelState {
    pub volume: i8,
    pub period: u16,
    pub finetune: i8,
    pub volume_slide: i8,

    pub portamento: i8,

    pub restart_sample_every: u8,
    pub cut_sample_after: u8,
}

impl ChannelState {
    pub fn new() -> Self {
        ChannelState {
            volume: 64,
            period: 0,
            finetune: 0,
            volume_slide: 0,

            portamento: 0,

            restart_sample_every: 0,
            cut_sample_after: 0,
        }
    }
}
