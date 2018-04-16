use std::io::{self, BufRead, BufReader};
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;

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
struct Chunk(Vec<Word>);

impl Chunk {
    fn new1(word: Word) -> Self {
        Chunk(vec![word])
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

    pub fn load_dict<R: BufRead>(&mut self, chars_dict: &mut R, words_dict: &mut R) -> io::Result<()> {
        let mut buf = String::new();
        while chars_dict.read_line(&mut buf)? > 0 {
            {
                let parts: Vec<&str> = buf.split(' ').collect();
                let freq: u32 = parts[0].parse().unwrap();
                let chr = parts[1].to_string();
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
                let chr = parts[1].to_string();
                if word_len > self.max_word_len {
                    self.max_word_len = word_len;
                }
                self.words.insert(chr, 0);
            }
            buf.clear();
        }
        Ok(())
    }

    pub fn load_dict_file<P: AsRef<Path>>(&mut self, chars_dict: P, words_dict: P) -> io::Result<()> {
        let chars_dict = File::open(chars_dict.as_ref())?;
        let words_dict = File::open(words_dict.as_ref())?;
        self.load_dict(&mut BufReader::new(chars_dict), &mut BufReader::new(words_dict))
    }

    pub fn cut(&self, text: &str) -> Vec<String> {
        let mut pos = 0;
        let chars: Vec<char> = text.chars().collect();
        let text_len = chars.len();
        let mut ret = Vec::new();
        while pos < text_len {
            let chr = chars[pos];
            let token = if is_chinese_char(chr) {
                self.get_chinese_words(&chars, &mut pos)
            } else {
                self.get_ascii_words(&chars, &mut pos)
            };
            if token.len() > 0 {
                ret.push(token);
            }
        }
        ret
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

    fn get_chinese_words(&self, chars: &[char], pos: &mut usize) -> String {
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
        words
    }

    fn create_simple_chunks(&self, chars: &[char], pos: &mut usize) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let words = self.get_match_chinese_words(chars, pos);
        for word in words {
            chunks.push(Chunk::new1(word));
        }
        chunks
    }
}

fn is_chinese_char(chr: char) -> bool {
    let chr = chr as u32;
    chr >= 0x4e00 && chr < 0x9fa6
}
