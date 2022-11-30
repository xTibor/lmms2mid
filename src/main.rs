use std::collections::HashMap;
use std::path::PathBuf;

mod lmms_model;
use lmms_model::{LmmsProject, LMMS_TICKS_PER_BAR};

use clap::Parser;
use midly::num::{u15, u24, u28, u4, u7};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
};

const MIDI_CC_BANK_SELECT_COARSE: u8 = 0;
const MIDI_CC_BANK_SELECT_FINE: u8 = 32;
const MIDI_CC_VOLUME: u8 = 7;
const MIDI_CC_PANNING: u8 = 10;

const MIDI_MAX_POLYPHONY: usize = 24;

/// A less broken MIDI-exporter for LMMS
#[derive(Debug, Parser)]
#[clap(author, version)]
struct Args {
    /// Input LMMS project file (.mmpz)
    input_path: PathBuf,

    /// Output MIDI file (.mid)
    output_path: PathBuf,

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

pub trait TrackEventKindExt {
    fn is_note_on(&self) -> bool;

    fn is_note_off(&self) -> bool;
}

impl TrackEventKindExt for TrackEventKind<'_> {
    fn is_note_on(&self) -> bool {
        matches!(
            self,
            TrackEventKind::Midi {
                message: MidiMessage::NoteOn { .. },
                ..
            }
        )
    }

    fn is_note_off(&self) -> bool {
        matches!(
            self,
            TrackEventKind::Midi {
                message: MidiMessage::NoteOff { .. },
                ..
            }
        )
    }
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
        Timing::Metrical(u15::from((LMMS_TICKS_PER_BAR / 4) as u16)),
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

    for (midi_channel, lmms_track) in &lmms_track_midi_channel {
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::MidiChannel(*midi_channel)),
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

        // Bank and preset selection
        {
            let bank = lmms_track.sf2_player().bank;
            let bank_coarse = u7::from((bank >> 7) as u8);
            let bank_fine = u7::from((bank & 0x7F) as u8);

            midi_track.push(TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Midi {
                    channel: *midi_channel,
                    message: MidiMessage::Controller {
                        controller: u7::from(MIDI_CC_BANK_SELECT_COARSE),
                        value: bank_coarse,
                    },
                },
            });

            midi_track.push(TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Midi {
                    channel: *midi_channel,
                    message: MidiMessage::Controller {
                        controller: u7::from(MIDI_CC_BANK_SELECT_FINE),
                        value: bank_fine,
                    },
                },
            });

            midi_track.push(TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Midi {
                    channel: *midi_channel,
                    message: MidiMessage::ProgramChange {
                        program: u7::from(lmms_track.sf2_player().patch as u8),
                    },
                },
            });
        }

        // Volume
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: *midi_channel,
                message: MidiMessage::Controller {
                    controller: u7::from(MIDI_CC_VOLUME),
                    value: u7::from(127), // TODO: remap
                },
            },
        });

        // Panning
        midi_track.push(TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: *midi_channel,
                message: MidiMessage::Controller {
                    controller: u7::from(MIDI_CC_PANNING),
                    value: u7::from(64), // TODO: remap
                },
            },
        });
    }

    let mut midi_track_events = Vec::new();

    for (midi_channel, lmms_track) in &lmms_track_midi_channel {
        for lmms_pattern in &lmms_track.patterns {
            for lmms_note in &lmms_pattern.notes {
                let ticks_start = lmms_pattern.position + lmms_note.position;
                let ticks_end = ticks_start + lmms_note.length;

                let mut note_key = lmms_note.key as isize;
                note_key += 69 - lmms_track.instrument_track.base_note as isize;

                if lmms_track.instrument_track.use_master_pitch == 1 {
                    note_key += lmms_project.head.master_pitch;
                };

                midi_track_events.push(AbsoluteTrackEvent {
                    ticks: ticks_start,
                    ticks_event_start: ticks_start,
                    kind: TrackEventKind::Midi {
                        channel: *midi_channel,
                        message: MidiMessage::NoteOn {
                            key: u7::from(note_key as u8),
                            vel: u7::from(127u8), // TODO: remap
                        },
                    },
                });

                midi_track_events.push(AbsoluteTrackEvent {
                    ticks: ticks_end,
                    ticks_event_start: ticks_start,
                    kind: TrackEventKind::Midi {
                        channel: *midi_channel,
                        message: MidiMessage::NoteOff {
                            key: u7::from(note_key as u8),
                            vel: u7::from(127u8), // TODO: remap
                        },
                    },
                });
            }
        }
    }

    midi_track_events.sort_by_key(
        |&AbsoluteTrackEvent {
             ticks,
             ticks_event_start,
             kind,
             ..
         }| {
            (
                ticks,
                ticks_event_start,
                !kind.is_note_on(),
                !kind.is_note_off(),
            )
        },
    );

    {
        let mut current_polyphony = 0;
        let mut already_warned = false;

        for event in midi_track_events.iter() {
            if event.kind.is_note_on() {
                current_polyphony += 1;

                if (current_polyphony > MIDI_MAX_POLYPHONY) && !already_warned {
                    eprintln!("warning: excessive polyphony at {}", event.ticks);
                    already_warned = true;
                }
            }

            if event.kind.is_note_off() {
                assert!(current_polyphony > 0);
                current_polyphony -= 1;

                if (current_polyphony <= MIDI_MAX_POLYPHONY) && already_warned {
                    already_warned = false;
                }
            }
        }
    }

    {
        let mut current_note_counts = HashMap::new();

        for event in midi_track_events.iter() {
            if let TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOn { key, .. },
            } = event.kind
            {
                let note_count = current_note_counts.entry((channel, key)).or_insert(0);
                *note_count += 1;

                if *note_count >= 2 {
                    eprintln!("warning: note overlap at {}", event.ticks);
                }
            }

            if let TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOff { key, .. },
            } = event.kind
            {
                let note_count = current_note_counts
                    .get_mut(&(channel, key))
                    .expect("failed to get note count");

                assert!(*note_count > 0);
                *note_count -= 1;

                if *note_count == 0 {
                    current_note_counts.remove(&(channel, key));
                }
            }
        }
    }

    for (event_index, event) in midi_track_events.iter().enumerate() {
        let delta_time = if event_index == 0 {
            event.ticks
        } else {
            let ticks_before = midi_track_events[event_index - 1].ticks;
            let ticks_current = midi_track_events[event_index].ticks;
            assert!(ticks_before <= ticks_current);
            ticks_current - ticks_before
        };

        midi_track.push(TrackEvent {
            delta: u28::from(delta_time as u32),
            kind: event.kind,
        });
    }

    midi_track.push(TrackEvent {
        delta: u28::from(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    midi_document.tracks.push(midi_track);
    midi_document
        .save(args.output_path)
        .expect("Failed to save output MIDI file");
}
