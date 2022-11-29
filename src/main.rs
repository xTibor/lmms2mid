use std::path::PathBuf;

mod lmms_model;
use lmms_model::LmmsProject;

use clap::Parser;
use midly::num::{u15, u24, u28};
use midly::{Format, Header, MetaMessage, Smf, Timing, Track, TrackEvent, TrackEventKind};

/// A less broken MIDI-exporter for LMMS
#[derive(Debug, Parser)]
#[clap(author, version)]
struct Args {
    /// Input LMMS project file (.mmpz)
    input_path: PathBuf,

    /// Output MIDI file (.mid)
    output_path: PathBuf,

    /// Ticks per beat
    #[arg(short, long, default_value_t = 576)]
    ticks_per_beat: usize,
}

// cargo run --release -- test/test.mmpz tmp/test.mid

fn main() {
    let args = Args::parse();
    let lmms_project =
        LmmsProject::load_compressed(&args.input_path).expect("Failed to load LMMS project file");

    let mut midi_document = Smf::new(Header::new(
        Format::SingleTrack,
        Timing::Metrical(u15::from(args.ticks_per_beat as u16)),
    ));

    let mut midi_track = Track::new();

    midi_track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(
            (60_000_000.0 / lmms_project.head.bpm as f32) as u32,
        ))),
    });

    // TODO

    midi_track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    midi_document.tracks.push(midi_track);
    midi_document
        .save(args.output_path)
        .expect("Failed to save output MIDI file");
}
