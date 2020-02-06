#![allow(dead_code)]
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use byteorder::{BigEndian, ReadBytesExt};

use sample::Signal;

fn map_range(x: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}

#[derive(Debug)]
pub struct Sample {
    name: String,
    length: u32,
    finetune: i8,
    volume: u8,
    repeat_offset: u32,
    repeat_length: u32,
    data: Vec<u8>,
}

impl Sample {
    pub fn name(&self) -> &str { &self.name }
    pub fn finetune(&self) -> i8 { self.finetune }
    pub fn volume(&self) -> u8 { self.volume }

    pub fn length(&self) -> u32 { self.length }
    pub fn repeat_offset(&self) -> u32 { self.repeat_offset }
    pub fn repeat_length(&self) -> u32 { self.repeat_length }

    pub fn data(&self) -> &[u8] { &self.data }
    pub fn set_data(&mut self, buf: Vec<u8>) {
        self.data = buf;
    }

    pub fn from(cursor: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        let mut buf: [u8; 30] = [0; 30];
        cursor.read_exact(&mut buf)?;
        Ok((&buf).into())
    }
}

impl From<&[u8; 30]> for Sample {
    fn from(other: &[u8; 30]) -> Self {
        let mut len = 22;
        for i in 0..len {
            if other[i] == 0 {
                len = i;
                break;
            }
        }

        let name = String::from_utf8_lossy(&other[0..len]).into_owned();
        let mut cursor = Cursor::new(&other[22..30]);

        let length = cursor.read_u16::<BigEndian>().unwrap();
        let finetune = cursor.read_i8().unwrap();
        let volume = cursor.read_u8().unwrap();
        let repeat_offset = cursor.read_u16::<BigEndian>().unwrap();
        let repeat_length = cursor.read_u16::<BigEndian>().unwrap();

        Sample {
            name, volume,
            length: length as u32 * 2,
            repeat_offset: repeat_offset as u32 * 2,
            repeat_length: repeat_length as u32 * 2,
            finetune: (finetune & 0x07) - (finetune & 0x08),
            data: Vec::new()
        }
    }
}

#[derive(Clone, Copy)]
pub struct SampleCursor<'a> {
    sample: &'a Sample,
    offset: usize,
    repeating: bool
}

impl<'a> SampleCursor<'a> {
    pub fn from(sample: &'a Sample) -> Self {
        SampleCursor {
            sample,
            offset: 0,
            repeating: false,
        }
    }

    pub fn sample(& self) -> &'a Sample { self.sample }

    fn read_byte(&mut self) -> f32 {
        if  self.offset >= self.sample.length as usize
            ||(self.repeating && self.offset >= (self.sample.repeat_offset as usize) + (self.sample.repeat_length as usize)) {
            self.offset = self.sample.repeat_offset as usize;
            self.repeating = true;
        }
        self.offset += 1;
        map_range(self.sample.data[self.offset - 1] as f32, 0.0, 255.0, -1.0, 1.0)
    }
}

/*impl Read for SampleCursor<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut i = 0;
        while i < buf.len() {
            buf[i] = self.read_byte();
            i += 1;
        }
        Ok(i)
    }
}

impl Seek for SampleCursor<'_> {
    fn seek(&mut self, seek_from: SeekFrom) -> io::Result<u64> {
        let mut new_offset = self.offset;
        match seek_from {
            SeekFrom::Current(pos) => new_offset += pos as usize,
            SeekFrom::Start(pos) => new_offset = pos as usize,
            SeekFrom::End(pos) => new_offset = self.sample.length as usize + pos as usize,
        };

        if new_offset >= self.sample.length as usize {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        } else {
            self.offset = new_offset;
            Ok(new_offset as u64)
        }
    }
}*/

impl Signal for SampleCursor<'_> {
    type Frame = [f32; 1];

    fn next(&mut self) -> Self::Frame {
        [self.read_byte()]
    }
}
