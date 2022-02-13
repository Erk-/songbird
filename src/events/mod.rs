//! Events relating to tracks, timing, and other callers.
//!
//! ## Listening for events
//! Driver events in Songbird are composed of two parts:
//! * An [`Event`] to listen out for. These may be discrete events,
//!   or generated by timers.
//! * An [`EventHandler`] to be called on receipt of an event. As event
//!   handlers may be shared between several events, the handler is called
//!   with an [`EventContext`] describing which event was fired.
//!
//! Event handlers are registered using functions such as [`Driver::add_global_event`],
//! or [`TrackHandle::add_event`], or. Internally, these pairs are stored
//! as [`EventData`].
//!
//! ## `EventHandler` lifecycle
//! An event handler is essentially just an async function which may return
//! another type of event to listen out for (an `Option<Event>`). For instance,
//! [`Some(Event::Cancel)`] will remove that event listener, while `None` won't
//! change it at all.
//!
//! The exception is one-off events like [`Event::Delayed`], which remove themselves
//! after one call *unless* an [`Event`] override is returned.
//!
//! ## Global and local listeners
//! *Global* event listeners are those which are placed onto the [`Driver`],
//! while *local* event listeners are those which are placed on a
//! [`Track`]/[`Handle`].
//!
//! Track or timed events, when local, return a reference to the parent track.
//! When registered globally, they fire on a per-tick basis, returning references to
//! all relevant tracks in that 20ms window. Global/local timed events use a global
//! timer or a [track's playback time], respectively.
//!
//! [`CoreEvent`]s may only be registered globally.
//!
//! [`Event`]: Event
//! [`EventHandler`]: EventHandler
//! [`EventContext`]: EventContext
//! [`Driver::add_global_event`]: crate::driver::Driver::add_global_event
//! [`Driver`]: crate::driver::Driver::add_global_event
//! [`TrackHandle::add_event`]: crate::tracks::TrackHandle::add_event
//! [`Track`]: crate::tracks::Track::events
//! [`Handle`]: crate::tracks::TrackHandle::add_event
//! [`EventData`]: EventData
//! [`Some(Event::Cancel)`]: Event::Cancel
//! [`Event::Delayed`]: Event::Delayed
//! [track's playback time]: crate::tracks::TrackState::play_time
//! [`CoreEvent`]: CoreEvent

mod context;
mod core;
mod data;
mod store;
mod track;
mod untimed;

pub use self::{
    context::{context_data, EventContext},
    core::*,
    data::*,
    store::*,
    track::*,
    untimed::*,
};
pub(crate) use context::{internal_data, CoreContext};

use async_trait::async_trait;
use std::time::Duration;

/// Trait to handle an event which can be fired per-track, or globally.
///
/// These may be feasibly reused between several event sources.
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Respond to one received event.
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event>;
}

/// Classes of event which may occur, triggering a handler
/// at the local (track-specific) or global level.
///
/// Local time-based events rely upon the current playback
/// time of a track, and so will not fire if a track becomes paused
/// or stops. In case this is required, global events are a better
/// fit.
///
/// Event handlers themselves are described in [`EventData::new`].
///
/// [`EventData::new`]: EventData::new
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// Periodic events rely upon two parameters: a *period*
    /// and an optional *phase*.
    ///
    /// If the *phase* is `None`, then the event will first fire
    /// in one *period*. Periodic events repeat automatically
    /// so long as the `action` in [`EventData`] returns `None`.
    ///
    /// [`EventData`]: EventData
    Periodic(Duration, Option<Duration>),
    /// Delayed events rely upon a *delay* parameter, and
    /// fire one *delay* after the audio context processes them.
    ///
    /// Delayed events are automatically removed once fired,
    /// so long as the `action` in [`EventData`] returns `None`.
    ///
    /// [`EventData`]: EventData
    Delayed(Duration),
    /// Track events correspond to certain actions or changes
    /// of state, such as a track finishing, looping, or being
    /// manually stopped.
    ///
    /// Track events persist while the `action` in [`EventData`]
    /// returns `None`.
    ///
    /// [`EventData`]: EventData
    Track(TrackEvent),
    /// Core events
    ///
    /// Track events persist while the `action` in [`EventData`]
    /// returns `None`. Core events **must** be applied globally,
    /// as attaching them to a track is a no-op.
    ///
    /// [`EventData`]: EventData
    Core(CoreEvent),
    /// Cancels the event, if it was intended to persist.
    Cancel,
}

impl Event {
    pub(crate) fn is_global_only(&self) -> bool {
        matches!(self, Self::Core(_))
    }
}

impl From<TrackEvent> for Event {
    fn from(evt: TrackEvent) -> Self {
        Event::Track(evt)
    }
}

impl From<CoreEvent> for Event {
    fn from(evt: CoreEvent) -> Self {
        Event::Core(evt)
    }
}