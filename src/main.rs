use std::path::PathBuf;

mod lmms_model;
use lmms_model::LmmsProject;

use clap::Parser;
use midly::num::{u15, u24, u28, u4};
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

    /// Track name
    #[arg(long)]
    track_name: Option<String>,

    /// Track copyright
    #[arg(long)]
    track_copyright: Option<String>,

    /// Track comment
    #[arg(long)]
    track_comment: Option<String>,
}

// cargo run --release -- test/test.mmpz tmp/test.mid

pub struct AbsoluteTrackEvent<'a> {
    //// When this event occurs in absolute MIDI ticks
    pub ticks: usize,

    /// When this event really started (NoteOn for NoteOff events)
    pub ticks_event_start: usize,

    /// MIDI event data
    pub kind: TrackEventKind<'a>,
}

fn main() {
    let args = Args::parse();
    let lmms_project =
        LmmsProject::load_compressed(&args.input_path).expect("Failed to load LMMS project file");

    // Sanity check for LMMS instrument/percussion track counts
    {
        let lmms_sf2_instrument_track_count = lmms_project
            .sf2_tracks()
            .filter(|track| track.is_instrument_track())
            .count();

        if lmms_sf2_instrument_track_count > 15 {
            eprintln!("note: LMMS project has more SF2 instrument tracks than available MIDI channels ({lmms_sf2_instrument_track_count}/15)");
            eprintln!("note: unassignable instrument tracks will be dropped");
        }

        let lmms_sf2_percussion_track_count = lmms_project
            .sf2_tracks()
            .filter(|track| track.is_precussion_track())
            .count();

        if lmms_sf2_percussion_track_count > 1 {
            eprintln!("note: LMMS project should only have at most one SF2 percussion track (found {lmms_sf2_percussion_track_count} tracks)");
            eprintln!("note: unassignable percussion tracks will be dropped");
        }
    }

    // LMMS track -> MIDI channel assignment
    let track_channel_assignment = {
        let mut results = Vec::new();

        // Instrument tracks
        results.extend(
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15]
                .into_iter()
                .map(u4::from)
                .zip(
                    lmms_project
                        .sf2_tracks()
                        .filter(|track| track.is_instrument_track()),
                ),
        );

        // Percussion track
        results.extend(
            [9].into_iter().map(u4::from).zip(
                lmms_project
                    .sf2_tracks()
                    .filter(|track| track.is_precussion_track()),
            ),
        );

        results
    };

    let mut midi_document = Smf::new(Header::new(
        Format::SingleTrack,
        Timing::Metrical(u15::from(args.ticks_per_beat as u16)),
    ));

    let mut midi_track = Track::new();
    //let mut midi_track_events = Vec::new();

    if let Some(ref track_name) = args.track_name {
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::TrackName(track_name.as_bytes())),
        });
    }

    if let Some(ref track_copyright) = args.track_copyright {
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::Copyright(track_copyright.as_bytes())),
        });
    }

    if let Some(ref track_comment) = args.track_comment {
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::Text(track_comment.as_bytes())),
        });
    }

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
