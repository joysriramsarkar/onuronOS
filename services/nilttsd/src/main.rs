// services/nilttsd/src/main.rs — Bangla & Multilingual Text-to-Speech Synthesizer (nilttsd)
// Rule-based Bengali & English G2P (Grapheme-to-Phoneme) and formant/harmonic PCM audio synthesizer.
// Provides speech accessibility for OnuronOS screen reader (nilsr) and user apps.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

pub const TTS_SOCK_PATH: &str = "/run/nilos/tts.sock";
pub const TTS_CACHE_DIR: &str = "/data/audio/tts_cache";
pub const SAMPLE_RATE: u32 = 16000; // 16kHz 16-bit Mono

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SpeakRequest {
    pub text: String,
    #[serde(default = "default_lang")]
    pub lang: String, // "bn" or "en"
    #[serde(default = "default_pitch")]
    pub pitch: f32, // baseline pitch multiplier (1.0 = normal ~130Hz)
    #[serde(default = "default_speed")]
    pub speed: f32, // speed multiplier (1.0 = normal)
}

fn default_lang() -> String {
    "bn".to_string()
}
fn default_pitch() -> f32 {
    1.0
}
fn default_speed() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SpeakResponse {
    pub success: bool,
    pub phonemes: Vec<String>,
    pub sample_count: usize,
    pub duration_ms: u64,
    pub wav_path: Option<String>,
}

/// Structure representing a phonetic segment with formant frequencies
#[derive(Debug, Clone)]
pub struct PhonemeSegment {
    pub symbol: String,
    pub is_vowel: bool,
    pub duration_ms: f32,
    pub f1: f32, // First formant frequency (Hz)
    pub f2: f32, // Second formant frequency (Hz)
}

/// Converts Bengali Unicode text to a sequence of phonemes (G2P)
pub fn bengali_text_to_phonemes(text: &str) -> Vec<String> {
    let mut phonemes = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        let next_ch = chars.get(i + 1).copied();

        // Check for Bengali vowels
        match ch {
            'অ' => phonemes.push("O".to_string()),
            'আ' => phonemes.push("A".to_string()),
            'ই' | 'ঈ' => phonemes.push("I".to_string()),
            'উ' | 'ঊ' => phonemes.push("U".to_string()),
            'ঋ' => {
                phonemes.push("R".to_string());
                phonemes.push("I".to_string());
            }
            'এ' => phonemes.push("E".to_string()),
            'ঐ' => {
                phonemes.push("O".to_string());
                phonemes.push("I".to_string());
            }
            'ও' => phonemes.push("O".to_string()),
            'ঔ' => {
                phonemes.push("O".to_string());
                phonemes.push("U".to_string());
            }

            // Consonants
            'ক' => phonemes.push("k".to_string()),
            'খ' => phonemes.push("kh".to_string()),
            'গ' => phonemes.push("g".to_string()),
            'ঘ' => phonemes.push("gh".to_string()),
            'ঙ' => phonemes.push("ng".to_string()),
            'চ' => phonemes.push("ch".to_string()),
            'ছ' => phonemes.push("chh".to_string()),
            'জ' => phonemes.push("j".to_string()),
            'ঝ' => phonemes.push("jh".to_string()),
            'ঞ' => phonemes.push("ny".to_string()),
            'ট' => phonemes.push("t".to_string()),
            'ঠ' => phonemes.push("th".to_string()),
            'ড' => phonemes.push("d".to_string()),
            'ঢ' => phonemes.push("dh".to_string()),
            'ণ' => phonemes.push("n".to_string()),
            'ত' => phonemes.push("t".to_string()),
            'থ' => phonemes.push("th".to_string()),
            'দ' => phonemes.push("d".to_string()),
            'ধ' => phonemes.push("dh".to_string()),
            'ন' => phonemes.push("n".to_string()),
            'প' => phonemes.push("p".to_string()),
            'ফ' => phonemes.push("ph".to_string()),
            'ব' => phonemes.push("b".to_string()),
            'ভ' => phonemes.push("bh".to_string()),
            'ম' => phonemes.push("m".to_string()),
            'য' | 'য়' => phonemes.push("y".to_string()),
            'র' | 'ড়' => phonemes.push("r".to_string()),
            'ঢ়' => phonemes.push("rh".to_string()),
            'ল' => phonemes.push("l".to_string()),
            'শ' | 'ষ' | 'স' => phonemes.push("sh".to_string()),
            'হ' => phonemes.push("h".to_string()),
            'ৎ' => phonemes.push("t".to_string()),

            // Matras (Vowel signs attached to consonants)
            'া' => phonemes.push("A".to_string()),
            'ি' | 'ী' => phonemes.push("I".to_string()),
            'ু' | 'ূ' => phonemes.push("U".to_string()),
            'ৃ' => {
                phonemes.push("R".to_string());
                phonemes.push("I".to_string());
            }
            'ে' => phonemes.push("E".to_string()),
            'ৈ' => {
                phonemes.push("O".to_string());
                phonemes.push("I".to_string());
            }
            'ো' => phonemes.push("O".to_string()),
            'ৌ' => {
                phonemes.push("O".to_string());
                phonemes.push("U".to_string());
            }

            // Modifiers
            '্' => {
                // Virama / Hasanta: suppress inherent vowel
            }
            'ং' => phonemes.push("ng".to_string()),
            'ঃ' => phonemes.push("h".to_string()),
            'ঁ' => phonemes.push("n".to_string()),
            ' ' => phonemes.push("SIL".to_string()), // Silence / pause
            _ => {
                // Ignore punctuation or non-Bengali symbols
            }
        }

        // If current char is a consonant and next is NOT a matra and NOT virama, insert inherent 'O'
        if is_bengali_consonant(ch) {
            match next_ch {
                Some('া') | Some('ি') | Some('ী') | Some('ু') | Some('ূ') | Some('ৃ')
                | Some('ে') | Some('ৈ') | Some('ো') | Some('ৌ') | Some('্') => {
                    // Handled by next matra or virama
                }
                _ => {
                    phonemes.push("O".to_string());
                }
            }
        }

        i += 1;
    }

    phonemes
}

fn is_bengali_consonant(ch: char) -> bool {
    matches!(
        ch,
        'ক' | 'খ' | 'গ' | 'ঘ' | 'ঙ'
            | 'চ' | 'ছ' | 'জ' | 'ঝ' | 'ঞ'
            | 'ট' | 'ঠ' | 'ড' | 'ঢ' | 'ণ'
            | 'ত' | 'থ' | 'দ' | 'ধ' | 'ন'
            | 'প' | 'ফ' | 'ব' | 'ভ' | 'ম'
            | 'য' | 'র' | 'ল' | 'শ' | 'ষ'
            | 'স' | 'হ' | 'ড়' | 'ঢ়' | 'য়' | 'ৎ'
    )
}

/// Converts English text to a simple phoneme sequence
pub fn english_text_to_phonemes(text: &str) -> Vec<String> {
    let mut phonemes = Vec::new();
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        let next_ch = chars.get(i + 1).copied();

        // Check digraphs first
        if let Some(next) = next_ch {
            match (ch, next) {
                ('t', 'h') => {
                    phonemes.push("th".to_string());
                    i += 2;
                    continue;
                }
                ('s', 'h') => {
                    phonemes.push("sh".to_string());
                    i += 2;
                    continue;
                }
                ('c', 'h') => {
                    phonemes.push("ch".to_string());
                    i += 2;
                    continue;
                }
                ('p', 'h') => {
                    phonemes.push("ph".to_string());
                    i += 2;
                    continue;
                }
                ('e', 'e') => {
                    phonemes.push("I".to_string());
                    i += 2;
                    continue;
                }
                ('o', 'o') => {
                    phonemes.push("U".to_string());
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }

        match ch {
            'a' => phonemes.push("A".to_string()),
            'e' => phonemes.push("E".to_string()),
            'i' => phonemes.push("I".to_string()),
            'o' => phonemes.push("O".to_string()),
            'u' => phonemes.push("U".to_string()),
            'b' => phonemes.push("b".to_string()),
            'c' | 'k' => phonemes.push("k".to_string()),
            'd' => phonemes.push("d".to_string()),
            'f' => phonemes.push("ph".to_string()),
            'g' => phonemes.push("g".to_string()),
            'h' => phonemes.push("h".to_string()),
            'j' => phonemes.push("j".to_string()),
            'l' => phonemes.push("l".to_string()),
            'm' => phonemes.push("m".to_string()),
            'n' => phonemes.push("n".to_string()),
            'p' => phonemes.push("p".to_string()),
            'r' => phonemes.push("r".to_string()),
            's' => phonemes.push("s".to_string()),
            't' => phonemes.push("t".to_string()),
            'v' | 'w' => phonemes.push("b".to_string()),
            'y' => phonemes.push("y".to_string()),
            ' ' => phonemes.push("SIL".to_string()),
            _ => {}
        }
        i += 1;
    }

    phonemes
}

/// Maps phoneme symbols to acoustic segments (duration and formants)
pub fn phoneme_to_segment(symbol: &str) -> PhonemeSegment {
    match symbol {
        "A" => PhonemeSegment {
            symbol: symbol.to_string(),
            is_vowel: true,
            duration_ms: 120.0,
            f1: 800.0,
            f2: 1250.0,
        },
        "I" => PhonemeSegment {
            symbol: symbol.to_string(),
            is_vowel: true,
            duration_ms: 100.0,
            f1: 300.0,
            f2: 2400.0,
        },
        "U" => PhonemeSegment {
            symbol: symbol.to_string(),
            is_vowel: true,
            duration_ms: 110.0,
            f1: 350.0,
            f2: 850.0,
        },
        "E" => PhonemeSegment {
            symbol: symbol.to_string(),
            is_vowel: true,
            duration_ms: 110.0,
            f1: 500.0,
            f2: 1800.0,
        },
        "O" => PhonemeSegment {
            symbol: symbol.to_string(),
            is_vowel: true,
            duration_ms: 110.0,
            f1: 500.0,
            f2: 1000.0,
        },
        "SIL" => PhonemeSegment {
            symbol: symbol.to_string(),
            is_vowel: false,
            duration_ms: 80.0,
            f1: 0.0,
            f2: 0.0,
        },
        // Consonants
        _ => PhonemeSegment {
            symbol: symbol.to_string(),
            is_vowel: false,
            duration_ms: 50.0,
            f1: 250.0,
            f2: 1500.0,
        },
    }
}

/// Generates 16kHz 16-bit signed PCM audio samples for a sequence of phonemes
pub fn synthesize_phonemes_to_pcm(phonemes: &[String], pitch_mult: f32, speed_mult: f32) -> Vec<i16> {
    let mut samples = Vec::new();
    let speed = if speed_mult > 0.1 { speed_mult } else { 1.0 };
    let f0 = 130.0 * (if pitch_mult > 0.2 { pitch_mult } else { 1.0 });

    for sym in phonemes {
        let seg = phoneme_to_segment(sym);
        let duration_sec = (seg.duration_ms / 1000.0) / speed;
        let num_samples = (duration_sec * SAMPLE_RATE as f32) as usize;

        if seg.symbol == "SIL" {
            // Silence
            samples.extend(std::iter::repeat(0i16).take(num_samples));
            continue;
        }

        for n in 0..num_samples {
            let t = n as f32 / SAMPLE_RATE as f32;
            let val = if seg.is_vowel {
                // Dual formant harmonic synthesis + base pitch
                let f0_wave = (2.0 * std::f32::consts::PI * f0 * t).sin();
                let f1_wave = (2.0 * std::f32::consts::PI * seg.f1 * t).sin() * 0.4;
                let f2_wave = (2.0 * std::f32::consts::PI * seg.f2 * t).sin() * 0.25;
                (f0_wave * 0.35 + f1_wave + f2_wave) * 0.8
            } else {
                // Consonant burst / noise
                let carrier = (2.0 * std::f32::consts::PI * seg.f1 * t).sin() * 0.4;
                let noise = ((n * 1337 + 7) % 100) as f32 / 100.0 - 0.5;
                (carrier + noise * 0.3) * 0.6
            };

            // Envelope (smooth attack and decay)
            let env = if n < 80 {
                n as f32 / 80.0
            } else if n > num_samples.saturating_sub(80) {
                (num_samples - n) as f32 / 80.0
            } else {
                1.0
            };

            let sample_i16 = (val * env * 14000.0).clamp(-32000.0, 32000.0) as i16;
            samples.push(sample_i16);
        }
    }

    samples
}

/// Generates standard 44-byte RIFF/WAVE header and writes complete WAV audio file
pub fn save_pcm_to_wav(pcm_samples: &[i16], out_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut file = fs::File::create(out_path)?;
    let num_samples = pcm_samples.len() as u32;
    let byte_rate = SAMPLE_RATE * 2; // 1 channel * 16 bits = 2 bytes per sample
    let block_align = 2u16;
    let data_chunk_size = num_samples * 2;
    let riff_chunk_size = 36 + data_chunk_size;

    // RIFF Header
    file.write_all(b"RIFF")?;
    file.write_all(&riff_chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt subchunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk1Size (16 for PCM)
    file.write_all(&1u16.to_le_bytes())?;  // AudioFormat (1 = PCM)
    file.write_all(&1u16.to_le_bytes())?;  // NumChannels (1 = Mono)
    file.write_all(&SAMPLE_RATE.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?; // BitsPerSample

    // data subchunk
    file.write_all(b"data")?;
    file.write_all(&data_chunk_size.to_le_bytes())?;

    // Samples
    for &sample in pcm_samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    file.flush()?;
    Ok(())
}

/// Executes speech synthesis for request and optionally writes WAV file
pub fn synthesize_speech(req: &SpeakRequest, cache_dir: Option<&Path>) -> SpeakResponse {
    let phonemes = if req.lang == "en" {
        english_text_to_phonemes(&req.text)
    } else {
        bengali_text_to_phonemes(&req.text)
    };

    let pcm = synthesize_phonemes_to_pcm(&phonemes, req.pitch, req.speed);
    let sample_count = pcm.len();
    let duration_ms = ((sample_count as f64 / SAMPLE_RATE as f64) * 1000.0) as u64;

    let wav_path = if let Some(dir) = cache_dir {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let file_path = dir.join(format!("tts_{}.wav", ts));
        if save_pcm_to_wav(&pcm, &file_path).is_ok() {
            Some(file_path.to_string_lossy().to_string())
        } else {
            None
        }
    } else {
        None
    };

    SpeakResponse {
        success: true,
        phonemes,
        sample_count,
        duration_ms,
        wav_path,
    }
}

fn main() {
    println!("[nilttsd] Bangla & Multilingual TTS Voice Synthesizer starting...");
    let cache_dir = Path::new(TTS_CACHE_DIR);
    let _ = fs::create_dir_all(cache_dir);

    // Initial greeting test synthesis
    let greeting = SpeakRequest {
        text: "অনুরন ওএস চালু হয়েছে।".to_string(),
        lang: "bn".to_string(),
        pitch: 1.0,
        speed: 1.0,
    };

    let resp = synthesize_speech(&greeting, Some(cache_dir));
    println!(
        "[nilttsd] Synthesized '{}': {} phonemes, {} samples, duration: {}ms, wav: {:?}",
        greeting.text,
        resp.phonemes.len(),
        resp.sample_count,
        resp.duration_ms,
        resp.wav_path
    );

    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bengali_g2p_basic() {
        let text = "অনুরন";
        let phonemes = bengali_text_to_phonemes(text);
        // অ -> O, ন -> n + ু -> U, র -> r + inherent O, ন -> n + inherent O
        assert!(phonemes.contains(&"O".to_string()));
        assert!(phonemes.contains(&"n".to_string()));
        assert!(phonemes.contains(&"U".to_string()));
        assert!(phonemes.contains(&"r".to_string()));
    }

    #[test]
    fn test_bengali_g2p_matras() {
        let text = "বাংলা";
        let phonemes = bengali_text_to_phonemes(text);
        // ব + া (A) + ং (ng) + ল + া (A)
        assert_eq!(phonemes, vec!["b", "A", "ng", "l", "A"]);
    }

    #[test]
    fn test_english_g2p() {
        let text = "Hello Onuron";
        let phonemes = english_text_to_phonemes(text);
        assert!(phonemes.contains(&"h".to_string()));
        assert!(phonemes.contains(&"E".to_string()));
        assert!(phonemes.contains(&"l".to_string()));
        assert!(phonemes.contains(&"SIL".to_string()));
    }

    #[test]
    fn test_synthesize_pcm_and_wav_generation() {
        let phonemes = vec!["b".to_string(), "A".to_string(), "ng".to_string(), "l".to_string(), "A".to_string()];
        let pcm = synthesize_phonemes_to_pcm(&phonemes, 1.0, 1.0);
        assert!(!pcm.is_empty());
        assert!(pcm.iter().any(|&s| s != 0));

        let tmp_path = std::env::temp_dir().join(format!("test_tts_{}.wav", std::process::id()));
        save_pcm_to_wav(&pcm, &tmp_path).expect("Failed to write WAV");
        assert!(tmp_path.exists());

        // Verify WAV header
        let bytes = fs::read(&tmp_path).expect("Read WAV");
        assert!(bytes.len() >= 44);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(&bytes[36..40], b"data");

        let _ = fs::remove_file(tmp_path);
    }
}
