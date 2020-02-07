use crate::notes::Note;

#[derive(Clone, Copy)]
pub struct ChannelState {
    pub volume: i8,
    pub period: u16,
    pub original_period: u16,
    pub finetune: i8,
    pub volume_slide: i8,

    pub arpeggio: (u8, u8),
    pub portamento: i8,
    pub slide_to_note: Option<(Note, u8)>,

    pub restart_sample_every: u8,
    pub cut_sample_after: u8,
}

impl ChannelState {
    pub fn new() -> Self {
        ChannelState {
            volume: 64,
            period: 0,
            original_period: 0,
            finetune: 0,
            volume_slide: 0,

            arpeggio: (0, 0),
            portamento: 0,
            slide_to_note: None,

            restart_sample_every: 0,
            cut_sample_after: 0,
        }
    }
}
