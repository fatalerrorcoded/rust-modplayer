use std::io::{Cursor, Read};
use byteorder::{BigEndian, ReadBytesExt};

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
