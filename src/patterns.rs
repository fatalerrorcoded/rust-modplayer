use std::ops::{Deref, DerefMut};
use std::{fmt, io::Cursor};
use std::io::Read;
use std::mem::size_of;

use byteorder::{BigEndian, ReadBytesExt};
use arr_macro::arr;

const LINES_PER_PATTERN: usize = 64;

// Pattern
#[repr(transparent)]
pub struct Pattern([PatternLine; LINES_PER_PATTERN]);

impl Deref for Pattern {
    type Target = [PatternLine];
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Pattern {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<&[u8]> for Pattern {
    fn from(buf: &[u8]) -> Self {
        let mut cursor = Cursor::new(buf);
        Pattern(
            arr![{
                let mut buf = [0; size_of::<PatternLine>()];
                cursor.read_exact(&mut buf).unwrap();
                PatternLine::from(&buf[..])
            }; 64]
        )
    }
}

// Pattern Line
#[derive(Debug)]
#[repr(transparent)]
pub struct PatternLine([PatternChannel; 4]);

impl Deref for PatternLine {
    type Target = [PatternChannel];
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for PatternLine {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<&[u8]> for PatternLine {
    fn from(buf: &[u8]) -> Self {
        let mut cursor = Cursor::new(buf);
        PatternLine(
            arr![PatternChannel(cursor.read_u32::<BigEndian>().unwrap()); 4]
        )
    }
}

// Pattern Channel
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PatternChannel(u32);

impl PatternChannel {
    pub fn number(&self) -> u8 {
        ((self.0 & 0xf0000000) >> 24) as u8 | ((self.0 & 0xf000) >> 12) as u8
    }

    pub fn period(&self) -> u16 {
        ((self.0 & 0x0fff0000) >> 16) as u16
    }

    pub fn effect(&self) -> ChannelEffect {
        ChannelEffect((self.0 & 0x0fff) as u16)
    }
}

impl Deref for PatternChannel {
    type Target = u32;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for PatternChannel {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl fmt::Debug for PatternChannel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PatternChannel {{ number: {}, period: {}, effect: {:?} }}",
            self.number(), self.period(), self.effect())
    }
}

// Effects
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ChannelEffect(u16);

impl ChannelEffect {
    pub fn number(&self) -> u8 {
        ((self.0 & 0x0f00) >> 8) as u8
    }

    pub fn arg_joined(&self) -> u8 {
        (self.0 & 0xff) as u8
    }

    pub fn arg_1(&self) -> u8 {
        ((self.0 & 0xf0) >> 4) as u8
    }

    pub fn arg_2(&self) -> u8 {
        (self.0 & 0x0f) as u8
    }
}

impl fmt::Debug for ChannelEffect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.number() == 0xe {
            write!(f, "ChannelEffect {{ number: {:x}, arg: {:x} }}",
                self.number() << 4 | self.arg_1(), self.arg_2())
        } else {
            write!(f, "ChannelEffect {{ number: {:x}, arg: {:x} }}",
                self.number(), self.arg_joined())
        }
    }
}
