use std::path::PathBuf;

mod lmms_model;
use lmms_model::LmmsProject;

use clap::Parser;
use midly::num::{u15, u24, u28, u4, u7};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
};

const MIDI_CC_BANK_SELECT: u8 = 0;
const MIDI_CC_VOLUME: u8 = 7;
const MIDI_CC_PANNING: u8 = 10;

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
            .filter(|lmms_track| lmms_track.is_instrument_track())
            .count();

        if lmms_sf2_instrument_track_count > 15 {
            eprintln!("note: LMMS project has more SF2 instrument tracks than available MIDI channels ({lmms_sf2_instrument_track_count}/15)");
            eprintln!("note: unassignable instrument tracks will be dropped");
        }

        let lmms_sf2_percussion_track_count = lmms_project
            .sf2_tracks()
            .filter(|lmms_track| lmms_track.is_precussion_track())
            .count();

        if lmms_sf2_percussion_track_count > 1 {
            eprintln!("note: LMMS project should only have at most one SF2 percussion track (found {lmms_sf2_percussion_track_count} tracks)");
            eprintln!("note: unassignable percussion tracks will be dropped");
        }
    }

    // LMMS track -> MIDI channel assignment
    let lmms_track_midi_channel = {
        let mut results = Vec::new();

        // Instrument tracks
        results.extend(
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15]
                .into_iter()
                .map(u4::from)
                .zip(
                    lmms_project
                        .sf2_tracks()
                        .filter(|lmms_track| lmms_track.is_instrument_track()),
                ),
        );

        // Percussion track
        results.extend(
            [9].into_iter().map(u4::from).zip(
                lmms_project
                    .sf2_tracks()
                    .filter(|lmms_track| lmms_track.is_precussion_track()),
            ),
        );

        results.sort_by_key(|(midi_channel, _lmms_track)| *midi_channel);
        results
    };

    let mut midi_document = Smf::new(Header::new(
        Format::SingleTrack,
        Timing::Metrical(u15::from(args.ticks_per_beat as u16)),
    ));

    let mut midi_track = Track::new();

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

    // MIDI channel initialization

    for (midi_channel, lmms_track) in lmms_track_midi_channel {
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::MidiChannel(midi_channel)),
        });

        if !lmms_track.name.is_empty() {
            if !lmms_track.name.is_ascii() {
                eprintln!(
                    "warning: non-ASCII LMMS track name '{}'",
                    lmms_track.name.escape_default(),
                );
                eprintln!("note: these track names may be mishandled by other music software");
            }

            midi_track.push(TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Meta(MetaMessage::InstrumentName(lmms_track.name.as_bytes())),
            });
        }

        // Bank selection
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: midi_channel,
                message: MidiMessage::Controller {
                    controller: u7::from(MIDI_CC_BANK_SELECT),
                    value: u7::from(lmms_track.sf2_player().bank as u8),
                },
            },
        });

        // Preset selection
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: midi_channel,
                message: MidiMessage::ProgramChange {
                    program: u7::from(lmms_track.sf2_player().patch as u8),
                },
            },
        });

        // Volume
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: midi_channel,
                message: MidiMessage::Controller {
                    controller: u7::from(MIDI_CC_VOLUME),
                    value: u7::from(0), // TODO: remap
                },
            },
        });

        // Panning
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: midi_channel,
                message: MidiMessage::Controller {
                    controller: u7::from(MIDI_CC_PANNING),
                    value: u7::from(0), // TODO: remap
                },
            },
        });
    }

    //let mut midi_track_events = Vec::new();

    midi_track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    midi_document.tracks.push(midi_track);
    midi_document
        .save(args.output_path)
        .expect("Failed to save output MIDI file");
}
