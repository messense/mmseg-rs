use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use arrayvec::ArrayVec;

#[cfg(feature = "embed-dict")]
static CHARS_DICT: &str = include_str!("chars.dic");
#[cfg(feature = "embed-dict")]
static WORDS_DICT: &str = include_str!("words.dic");

#[derive(Debug, Clone)]
struct Word {
    text: String,
    freq: u32,
    len: u32,
}

#[derive(Debug)]
struct Chunk(ArrayVec<Word, 3>);

impl Chunk {
    #[inline]
    fn new1(word: Word) -> Self {
        let mut vec = ArrayVec::new();
        unsafe { vec.push_unchecked(word) };
        Chunk(vec)
    }

    #[inline]
    fn new2(word1: Word, word2: Word) -> Self {
        let mut vec = ArrayVec::new();
        unsafe {
            vec.push_unchecked(word1);
            vec.push_unchecked(word2);
        }
        Chunk(vec)
    }

    #[inline]
    fn new3(word1: Word, word2: Word, word3: Word) -> Self {
        let mut vec = ArrayVec::new();
        unsafe {
            vec.push_unchecked(word1);
            vec.push_unchecked(word2);
            vec.push_unchecked(word3);
        }
        Chunk(vec)
    }

    fn total_word_len(&self) -> u32 {
        let mut len = 0;
        for word in &self.0 {
            len += word.len;
        }
        len
    }

    fn avg_word_len(&self) -> f32 {
        self.total_word_len() as f32 / self.0.len() as f32
    }

    fn stddev(&self) -> f32 {
        let avg = self.avg_word_len();
        let mut sum = 0f32;
        for word in &self.0 {
            let tmp = word.len as f32 - avg;
            sum += tmp.powi(2);
        }
        (sum / self.0.len() as f32).sqrt()
    }

    fn word_freq(&self) -> f32 {
        let mut sum = 0f32;
        for word in &self.0 {
            if word.len == 1 {
                sum += (word.freq as f32).ln();
            }
        }
        sum
    }
}

#[derive(Debug)]
pub struct MMSeg {
    words: HashMap<String, u32>,
    max_word_len: u32,
}

impl MMSeg {
    pub fn new() -> Self {
        let mut seg = Self {
            words: HashMap::new(),
            max_word_len: 0,
        };
        #[cfg(feature = "embed-dict")]
        seg.load_embed_dict().unwrap();
        seg
    }

    #[cfg(feature = "embed-dict")]
    fn load_embed_dict(&mut self) -> io::Result<()> {
        let mut chars_dict = BufReader::new(CHARS_DICT.as_bytes());
        let mut words_dict = BufReader::new(WORDS_DICT.as_bytes());
        self.load_dict(&mut chars_dict, &mut words_dict)
    }

    pub fn load_dict<R: BufRead>(
        &mut self,
        chars_dict: &mut R,
        words_dict: &mut R,
    ) -> io::Result<()> {
        let mut buf = String::new();
        while chars_dict.read_line(&mut buf)? > 0 {
            {
                let parts: Vec<&str> = buf.split(' ').collect();
                let freq: u32 = parts[0].parse().unwrap();
                let chr = parts[1].trim().to_string();
                let word_len = chr.chars().count() as u32;
                if word_len > self.max_word_len {
                    self.max_word_len = word_len;
                }
                self.words.insert(chr, freq);
            }
            buf.clear();
        }
        while words_dict.read_line(&mut buf)? > 0 {
            {
                let parts: Vec<&str> = buf.split(' ').collect();
                let word_len: u32 = parts[0].parse().unwrap();
                let chr = parts[1].trim().to_string();
                if word_len > self.max_word_len {
                    self.max_word_len = word_len;
                }
                self.words.insert(chr, 0);
            }
            buf.clear();
        }
        Ok(())
    }

    pub fn load_dict_file<P: AsRef<Path>>(
        &mut self,
        chars_dict: P,
        words_dict: P,
    ) -> io::Result<()> {
        let chars_dict = File::open(chars_dict.as_ref())?;
        let words_dict = File::open(words_dict.as_ref())?;
        self.load_dict(
            &mut BufReader::new(chars_dict),
            &mut BufReader::new(words_dict),
        )
    }

    pub fn cut_simple(&self, text: &str) -> Vec<String> {
        self.cut_internal(text, true)
    }

    pub fn cut(&self, text: &str) -> Vec<String> {
        self.cut_internal(text, false)
    }

    #[inline]
    fn cut_internal(&self, text: &str, simple: bool) -> Vec<String> {
        let mut pos = 0;
        let chars: Vec<char> = text.chars().collect();
        let mut ret = Vec::new();
        loop {
            if let Some(token) = self.get_next_token(&chars, &mut pos, simple) {
                ret.push(token);
            } else {
                break;
            }
        }
        ret
    }

    fn get_next_token(&self, chars: &[char], pos: &mut usize, simple: bool) -> Option<String> {
        while *pos < chars.len() {
            let chr = chars[*pos];
            let token = if is_chinese_char(chr) {
                if simple {
                    self.get_chinese_words_simple(&chars, pos)
                } else {
                    self.get_chinese_words_complex(&chars, pos)
                }
            } else {
                self.get_ascii_words(&chars, pos)
            };
            if token.len() > 0 {
                return Some(token);
            }
        }
        None
    }

    fn get_ascii_words(&self, chars: &[char], pos: &mut usize) -> String {
        while *pos < chars.len() {
            let chr = chars[*pos];
            if chr.is_ascii_alphanumeric() || is_chinese_char(chr) {
                break;
            }
            *pos += 1;
        }
        let start = *pos;
        while *pos < chars.len() {
            let chr = chars[*pos];
            if !chr.is_ascii_alphanumeric() {
                break;
            }
            *pos += 1;
        }
        let end = *pos;

        // skip Chinese word whitespaces and punctuations
        while *pos < chars.len() {
            let chr = chars[*pos];
            if chr.is_ascii_alphanumeric() || is_chinese_char(chr) {
                break;
            }
            *pos += 1;
        }
        // FIXME: avoid allocation
        chars[start..end].iter().collect()
    }

    fn get_chinese_words_simple(&self, chars: &[char], pos: &mut usize) -> String {
        let chunks = self.create_simple_chunks(chars, pos);
        let result = chunks.into_iter().max_by_key(|chk| chk.total_word_len());
        if let Some(chunk) = result {
            let mut ret = String::new();
            for word in chunk.0 {
                if word.text.is_empty() {
                    continue;
                }
                *pos += word.len as usize;
                ret.push_str(&word.text);
            }
            return ret;
        }
        String::new()
    }

    fn get_chinese_words_complex(&self, chars: &[char], pos: &mut usize) -> String {
        fn take_high_test<F>(chunks: &mut [Chunk], mut compare: F) -> &mut [Chunk]
        where
            F: FnMut(&Chunk, &Chunk) -> Ordering,
        {
            let mut i = 1;
            for j in 1..chunks.len() {
                let rlt = compare(&chunks[j], &chunks[0]);
                if rlt == Ordering::Greater {
                    i = 0;
                }
                if rlt != Ordering::Less {
                    chunks.swap(i, j);
                    i += 1;
                }
            }
            &mut chunks[0..i]
        }

        let mut chunks = self.create_chunks(chars, pos);
        let mut chunks = take_high_test(&mut chunks, |a, b| {
            a.total_word_len().cmp(&b.total_word_len())
        });
        let mut chunks = take_high_test(&mut chunks, |a, b| {
            a.avg_word_len()
                .partial_cmp(&b.avg_word_len())
                .unwrap_or(Ordering::Equal)
        });
        let mut chunks = take_high_test(&mut chunks, |a, b| {
            b.stddev()
                .partial_cmp(&a.stddev())
                .unwrap_or(Ordering::Equal)
        });
        let chunks = take_high_test(&mut chunks, |a, b| {
            a.word_freq()
                .partial_cmp(&b.word_freq())
                .unwrap_or(Ordering::Equal)
        });
        let result = chunks.get(0);
        if let Some(chunk) = result {
            let mut ret = String::new();
            for word in chunk.0.iter().take(1) {
                if word.text.is_empty() {
                    continue;
                }
                *pos += word.len as usize;
                ret.push_str(&word.text);
            }
            return ret;
        }
        String::new()
    }

    fn get_match_chinese_words(&self, chars: &[char], pos: &mut usize) -> Vec<Word> {
        let mut words = Vec::new();
        let original_pos = *pos;
        let mut index = 0;
        while *pos < chars.len() {
            if index >= self.max_word_len {
                break;
            } else if !is_chinese_char(chars[*pos]) {
                break;
            }
            *pos += 1;
            index += 1;
            let text: String = chars[original_pos..*pos].iter().collect();
            let word = self.words.get(&text).map(|v| {
                let len = text.chars().count();
                Word {
                    text: text,
                    freq: *v,
                    len: len as u32,
                }
            });
            if let Some(word) = word {
                words.push(word);
            }
        }
        *pos = original_pos;
        if words.is_empty() {
            // if word not exists , place "X" and length 0
            words.push(Word {
                text: "".to_string(),
                freq: 0,
                len: 1,
            })
        }
        words
    }

    fn create_simple_chunks(&self, chars: &[char], pos: &mut usize) -> Vec<Chunk> {
        let words = self.get_match_chinese_words(chars, pos);
        let mut chunks = Vec::with_capacity(words.len());
        for word in words {
            if word.text.is_empty() {
                continue;
            }
            chunks.push(Chunk::new1(word));
        }
        chunks
    }

    fn create_chunks(&self, chars: &[char], pos: &mut usize) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let original_pos = *pos;
        let text_len = chars.len();
        let words1 = self.get_match_chinese_words(chars, pos);
        for word1 in words1 {
            let word1_len = word1.len as usize;
            *pos += word1_len;
            if *pos < text_len {
                let words2 = self.get_match_chinese_words(chars, pos);
                for word2 in words2 {
                    let word2_len = word2.len as usize;
                    *pos += word2_len;
                    if *pos < text_len {
                        let words3 = self.get_match_chinese_words(chars, pos);
                        for word3 in words3 {
                            if word3.text.is_empty() {
                                chunks.push(Chunk::new2(word1.clone(), word2.clone()));
                            } else {
                                chunks.push(Chunk::new3(word1.clone(), word2.clone(), word3));
                            }
                        }
                    } else if *pos == text_len {
                        chunks.push(Chunk::new2(word1.clone(), word2));
                    }
                    *pos -= word2_len;
                }
            } else if *pos == text_len {
                chunks.push(Chunk::new1(word1));
            }
            *pos -= word1_len;
        }
        *pos = original_pos;
        chunks
    }
}

fn is_chinese_char(chr: char) -> bool {
    let chr = chr as u32;
    chr >= 0x4e00 && chr < 0x9fa6
}
