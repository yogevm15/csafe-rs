use memchr::{Memchr, memchr_iter};

type Stuffed = &'static [u8; 2];

const ESCAPING: u8 = 0b1111_0011;
const F0: u8 = 0b1111_0000;
const F0_STUFFED: u8 = 0b0000_0000;
const F0_ESCAPED: Stuffed = &[ESCAPING, F0_STUFFED];
pub const F1: u8 = 0b1111_0001;
const F1_STUFFED: u8 = 0b0000_0001;
const F1_ESCAPED: Stuffed = &[ESCAPING, F1_STUFFED];
pub const F2: u8 = 0b1111_0010;
const F2_STUFFED: u8 = 0b0000_0010;
const F2_ESCAPED: Stuffed = &[ESCAPING, F2_STUFFED];
const F3: u8 = 0b1111_0011;
const F3_STUFFED: u8 = 0b0000_0011;
const F3_ESCAPED: Stuffed = &[ESCAPING, F3_STUFFED];

pub fn stuff(data: &[u8]) -> impl Iterator<Item=&[u8]> {
    Stuff::new(data)
}

struct Stuff<'a> {
    stuff: Option<Stuffed>,
    rest: Option<&'a [u8]>,
}

impl<'a> Stuff<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            stuff: None,
            rest: Some(data),
        }
    }
}

impl<'a> Iterator for Stuff<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(stuff) = self.stuff.take() {
            return Some(stuff);
        }

        let rest = self.rest.take()?;

        let Some((pos, stuff)) = rest.iter().enumerate().find_map(|(i, b)|match *b {
            F0 => Some(F0_ESCAPED),
            F1 => Some(F1_ESCAPED),
            F2 => Some(F2_ESCAPED),
            F3 => Some(F3_ESCAPED),
            _ => None,
        }.map(|s|(i ,s))) else {
            return Some(rest);
        };
        let (before, after) = rest.split_at(pos);
        self.stuff = Some(stuff);
        self.rest = Some(&after[1..]);
        Some(before)

    }
}


pub fn unstuff(data: &[u8]) -> impl Iterator<Item = &[u8]> {
    Unstuffed::new(data)
}

struct Unstuffed<'a> {
    data: &'a [u8],
    unstuff: Option<&'static [u8]>,
    rest: Option<&'a [u8]>,
    iter: Memchr<'a>,
}

impl<'a> Unstuffed<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            unstuff: None,
            rest: Some(data),
            iter: memchr_iter(ESCAPING, data),
        }
    }
}

impl<'a> Iterator for Unstuffed<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(unstuff) = self.unstuff.take() {
            return Some(unstuff);
        }

        let Some(escape) = self.iter.next() else {
            return self.rest.take();
        };

        let (before, after) = self.data.split_at(escape);
        self.unstuff = match after.get(1) {
            Some(&F0_STUFFED) => Some(&[F0]),
            Some(&F1_STUFFED) => Some(&[F1]),
            Some(&F2_STUFFED) => Some(&[F2]),
            Some(&F3_STUFFED) => Some(&[F3]),
            _ => panic!("Invalid byte after escape"),
        };
        self.rest = Some(&after[2..]);

        Some(before)
    }
}
