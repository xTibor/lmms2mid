use std::error::Error;
use std::path::Path;
use std::{fs, str};

use miniz_oxide::inflate::decompress_to_vec_zlib;
use strong_xml::XmlRead;

pub const LMMS_TICKS_PER_BEAT: usize = 12;

#[derive(Debug, XmlRead)]
#[xml(tag = "lmms-project")]
pub struct LmmsProject {
    #[xml(attr = "type")]
    pub r#type: String,

    #[xml(attr = "version")]
    pub version: usize,

    #[xml(attr = "creator")]
    pub creator: String,

    #[xml(attr = "creatorversion")]
    pub creator_version: String,

    #[xml(child = "head")]
    pub head: LmmsHead,

    #[xml(child = "song")]
    pub song: LmmsSong,
}

#[derive(Debug, XmlRead)]
#[xml(tag = "head")]
pub struct LmmsHead {
    #[xml(attr = "timesig_denominator")]
    pub time_signature_denominator: usize,

    #[xml(attr = "timesig_numerator")]
    pub time_signature_numerator: usize,

    #[xml(attr = "bpm")]
    pub bpm: usize,

    #[xml(attr = "masterpitch")]
    pub master_pitch: isize,

    #[xml(attr = "mastervol")]
    pub master_volume: usize,
}

#[derive(Debug, XmlRead)]
#[xml(tag = "song")]
pub struct LmmsSong {
    #[xml(child = "trackcontainer")]
    pub track_container: LmmsTrackContainer,
    // Skipped: track (automationtrack)
    // Skipped: fxmixer
    // Skipped: ControllerRackView
    // Skipped: pianoroll
    // Skipped: automationeditor
    // Skipped: projectnotes
    // Skipped: timeline
    // Skipped: controllers
}

#[derive(Debug, XmlRead)]
#[xml(tag = "trackcontainer")]
pub struct LmmsTrackContainer {
    #[xml(attr = "visible")]
    pub visible: usize,

    #[xml(attr = "minimized")]
    pub minimized: usize,

    #[xml(attr = "maximized")]
    pub maximized: usize,

    #[xml(attr = "x")]
    pub x: isize,

    #[xml(attr = "y")]
    pub y: isize,

    #[xml(attr = "width")]
    pub width: usize,

    #[xml(attr = "height")]
    pub height: usize,

    #[xml(attr = "type")]
    pub r#type: String,

    #[xml(child = "track")]
    pub tracks: Vec<LmmsTrack>,
}

#[derive(Debug, XmlRead)]
#[xml(tag = "track")]
pub struct LmmsTrack {
    #[xml(attr = "name")]
    pub name: String,

    #[xml(attr = "muted")]
    pub muted: usize,

    #[xml(attr = "mutedBeforeSolo")]
    pub muted_before_solo: usize,

    #[xml(attr = "type")]
    pub r#type: usize,

    #[xml(attr = "solo")]
    pub solo: usize,

    #[xml(child = "instrumenttrack")]
    pub instrument_track: LmmsInstrumentTrack,

    #[xml(child = "pattern")]
    pub patterns: Vec<LmmsPattern>,
}

#[derive(Debug, XmlRead)]
#[xml(tag = "instrumenttrack")]
pub struct LmmsInstrumentTrack {
    #[xml(attr = "vol")]
    pub volume: usize,

    #[xml(attr = "pan")]
    pub panning: isize,

    #[xml(attr = "pitchrange")]
    pub pitch_range: usize,

    #[xml(attr = "lastkey")]
    pub last_key: usize,

    #[xml(attr = "fxch")]
    pub fx_channel: usize,

    #[xml(attr = "usemasterpitch")]
    pub use_master_pitch: usize,

    #[xml(attr = "pitch")]
    pub pitch: f32,

    #[xml(attr = "firstkey")]
    pub first_key: usize,

    #[xml(attr = "basenote")]
    pub base_note: usize,

    #[xml(attr = "enablecc")]
    pub enable_cc: usize,

    #[xml(child = "instrument")]
    pub instrument: LmmsInstrument,
    // Skipped: midicontrollers
    // Skipped: eldata
    // Skipped: chordcreator
    // Skipped: arpeggiator
    // Skipped: midiport
    // Skipped: fxchain
}

#[derive(Debug, XmlRead)]
#[xml(tag = "instrument")]
pub struct LmmsInstrument {
    #[xml(attr = "name")]
    pub name: String,

    #[xml(child = "sf2player")]
    pub sf2_player: Option<LmmsSf2Player>,
}

#[derive(Debug, XmlRead)]
#[xml(tag = "sf2player")]
pub struct LmmsSf2Player {
    #[xml(attr = "src")]
    pub src: String,

    #[xml(attr = "bank")]
    pub bank: usize,

    #[xml(attr = "patch")]
    pub patch: usize,

    #[xml(attr = "gain")]
    pub gain: f32,

    #[xml(attr = "reverbOn")]
    pub reverb_on: usize,

    #[xml(attr = "reverbLevel")]
    pub reverb_level: f32,

    #[xml(attr = "reverbDamping")]
    pub reverb_damping: f32,

    #[xml(attr = "reverbWidth")]
    pub reverb_width: f32,

    #[xml(attr = "reverbRoomSize")]
    pub reverb_room_size: f32,

    #[xml(attr = "chorusOn")]
    pub chorus_on: usize,

    #[xml(attr = "chorusLevel")]
    pub chorus_level: f32,

    #[xml(attr = "chorusNum")]
    pub chorus_num: usize,

    #[xml(attr = "chorusDepth")]
    pub chorus_depth: f32,

    #[xml(attr = "chorusSpeed")]
    pub chorus_speed: f32,
}

#[derive(Debug, XmlRead)]
#[xml(tag = "pattern")]
pub struct LmmsPattern {
    #[xml(attr = "name")]
    pub name: String,

    #[xml(attr = "muted")]
    pub muted: usize,

    #[xml(attr = "pos")]
    pub position: usize,

    #[xml(attr = "steps")]
    pub steps: usize,

    #[xml(attr = "type")]
    pub r#type: usize,

    #[xml(child = "note")]
    pub notes: Vec<LmmsNote>,
}

#[derive(Debug, XmlRead)]
#[xml(tag = "note")]
pub struct LmmsNote {
    #[xml(attr = "vol")]
    pub volume: usize,

    #[xml(attr = "pan")]
    pub panning: isize,

    #[xml(attr = "pos")]
    pub position: usize,

    #[xml(attr = "len")]
    pub length: usize,

    #[xml(attr = "key")]
    pub key: usize,
}

impl LmmsProject {
    pub fn load_compressed(path: &Path) -> Result<Self, Box<dyn Error>> {
        let compressed_bin = fs::read(path)?;
        let uncompressed_bin = decompress_to_vec_zlib(&compressed_bin[4..])?;
        let uncompressed_xml = str::from_utf8(&uncompressed_bin)?;

        Ok(LmmsProject::from_str(uncompressed_xml)?)
    }

    pub fn sf2_tracks(&self) -> impl Iterator<Item = &LmmsTrack> {
        self.song
            .track_container
            .tracks
            .iter()
            .filter(|track| track.instrument_track.instrument.sf2_player.is_some())
    }
}

impl LmmsTrack {
    pub fn sf2_player(&self) -> &LmmsSf2Player {
        self.instrument_track
            .instrument
            .sf2_player
            .as_ref()
            .expect("Not an SF2 track")
    }

    pub fn is_instrument_track(&self) -> bool {
        self.sf2_player().bank != 128
    }

    pub fn is_precussion_track(&self) -> bool {
        self.sf2_player().bank == 128
    }
}
