use std::num::NonZeroU16;
use rand;
use rand::Rng;

use serde::{
    Serialize,
    Deserialize
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum BPM {
    Number(NonZeroU16)
}

impl BPM {
    pub fn random() -> BPM {
        let mut num = NonZeroU16::new(rand::thread_rng().gen_range(1..=300)).unwrap();
        BPM::Number(num)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSongListing {
    title: String,
    bpm: BPM
}

impl FileSongListing {
    pub fn random() -> FileSongListing {
        FileSongListing {
            title: format!("Song {}", rand::thread_rng().gen_range(1..=300)),
            bpm: BPM::random()
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn bpm(&self) -> BPM {
        self.bpm
    }
}