// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed errors of the pathing library (NFR-07).

/// An error from the pathing library. Messages are written for the user: they
/// name the field or the place in the toolpath and say what is wrong.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PathError {
    /// Shared data was invalid (a tool, a mesh, a profile value).
    #[error(transparent)]
    Data(openchuck_types::Error),
    /// An operation parameter is missing, unknown, of the wrong type or out
    /// of range (OP-16).
    #[error("parameter {name}: {reason}")]
    Parameter {
        /// The parameter's name in the schema.
        name: String,
        /// What is wrong with it.
        reason: String,
    },
    /// No strategy of that name is registered (OP-16).
    #[error("no strategy named \"{0}\" is available")]
    UnknownStrategy(String),
    /// The surface cannot be used (for example an empty mesh).
    #[error("surface: {0}")]
    Surface(String),
    /// A region on the unrolled development is invalid or cannot be offset
    /// (decision record 0016, SP-7): the message names the loop and the fault.
    #[error("region: {0}")]
    Region(String),
    /// The rotary axis cannot follow the toolpath within its limits (OP-15).
    #[error("A axis: {0}")]
    RotaryLimit(String),
    /// The strategy could not produce a toolpath from its inputs.
    #[error("{0}")]
    Strategy(String),
    /// A check refuses to generate (decision record 0644): its topic (one
    /// of [`crate::topic`]), what is wrong, why and what to change, and the
    /// key of the setting to change, if one does. The application shows it
    /// in the one form of decision record 0536 section 3, "<Topic>
    /// (operation '<name>'): <text>", and links to the setting.
    #[error("{topic}: {text}")]
    Refused {
        /// The check's topic.
        topic: String,
        /// What is wrong, why and what to change.
        text: String,
        /// The key of the operation's setting to change, if one does.
        setting: Option<String>,
    },
    /// The job was cancelled.
    #[error("the job was cancelled")]
    Cancelled,
}

impl PathError {
    /// A refusal under `topic` (decision record 0644).
    pub fn refused(topic: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Refused {
            topic: topic.into(),
            text: text.into(),
            setting: None,
        }
    }

    /// This refusal, naming the operation's setting `key` as the one to
    /// change. Other errors are returned as they are.
    #[must_use]
    pub fn with_setting(self, key: impl Into<String>) -> Self {
        match self {
            Self::Refused { topic, text, .. } => Self::Refused {
                topic,
                text,
                setting: Some(key.into()),
            },
            other => other,
        }
    }

    /// This error with every position in its text written as the work frame
    /// reads it (decision record 0699; [`crate::words::settle`]).
    #[must_use]
    pub(crate) fn settled(self, frame: openchuck_types::position::WorkFrame) -> Self {
        let settle = |text: String| crate::words::settle(&text, frame);
        match self {
            Self::Parameter { name, reason } => Self::Parameter {
                name,
                reason: settle(reason),
            },
            Self::UnknownStrategy(text) => Self::UnknownStrategy(settle(text)),
            Self::Surface(text) => Self::Surface(settle(text)),
            Self::Region(text) => Self::Region(settle(text)),
            Self::RotaryLimit(text) => Self::RotaryLimit(settle(text)),
            Self::Strategy(text) => Self::Strategy(settle(text)),
            Self::Refused {
                topic,
                text,
                setting,
            } => Self::Refused {
                topic,
                text: settle(text),
                setting,
            },
            other @ (Self::Data(_) | Self::Cancelled) => other,
        }
    }

    /// Shorthand for [`PathError::Parameter`].
    pub fn parameter(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Parameter {
            name: name.into(),
            reason: reason.into(),
        }
    }
}

/// Result type of the pathing library.
pub type Result<T> = std::result::Result<T, PathError>;

/// The shared cancel error becomes [`PathError::Cancelled`].
impl From<openchuck_types::Error> for PathError {
    fn from(error: openchuck_types::Error) -> Self {
        match error {
            openchuck_types::Error::Cancelled => Self::Cancelled,
            other => Self::Data(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn req_ui_12_a_refusal_carries_its_topic_and_setting() {
        // Decision record 0644: a refusal names its check and the setting
        // to change, so the application shows it in the one form.
        let refused =
            PathError::refused(crate::topic::END_FACING, "the stepdown must be above 0 mm")
                .with_setting("stepdown");
        assert_eq!(
            refused,
            PathError::Refused {
                topic: "End facing".into(),
                text: "the stepdown must be above 0 mm".into(),
                setting: Some("stepdown".into()),
            }
        );
        assert_eq!(
            refused.to_string(),
            "End facing: the stepdown must be above 0 mm"
        );
        // A setting names nothing on another error.
        assert_eq!(PathError::Cancelled.with_setting("x"), PathError::Cancelled);
    }

    /// Decision record 0699: a refusal that names a position writes it in
    /// the work frame, whichever kind of error carries it; an error with no
    /// position is returned as it is.
    #[test]
    fn req_ui_12_a_refusal_writes_its_positions_in_the_work_frame() {
        use crate::words::x;
        use openchuck_types::position::WorkFrame;
        let frame = WorkFrame::new(5.0);
        let at = format!("a move at {} mm would reach over the held region", x(38.0));
        let refused = PathError::refused(crate::topic::HELD_REGION, at.clone());
        assert_eq!(
            refused.settled(frame).to_string(),
            "Held region: a move at X 33.000 mm would reach over the held region"
        );
        for error in [
            PathError::parameter("start", at.clone()),
            PathError::Surface(at.clone()),
            PathError::Region(at.clone()),
            PathError::RotaryLimit(at.clone()),
            PathError::Strategy(at.clone()),
        ] {
            assert!(
                error
                    .clone()
                    .settled(frame)
                    .to_string()
                    .contains("X 33.000 mm"),
                "{error:?}"
            );
        }
        assert_eq!(PathError::Cancelled.settled(frame), PathError::Cancelled);
    }
}
