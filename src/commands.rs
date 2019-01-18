//! Ground control commands and responses
//!
//! Provides command validation based on the required command
//! state.

use clap::crate_version;
use std::fmt;

pub mod command {
    //! Namespace for raw string commands

    /// Prefix for all commands
    pub static PREFIX: &'static str = "--gc";
    /// User asks for help
    pub static HELP: &'static str = "--gc-help";
    /// User requests wingmen
    pub static WINGS: &'static str = "--gc-wings";
    /// User calls of their wingmen
    pub static CALL_OFF: &'static str = "--gc-call-off";
    /// Version of this program
    pub static VERSION: &'static str = "--gc-version";
}

/// Generate a string containing versioning info for this program
fn version_message() -> Vec<String> {
    vec![format!(
        "AIRMASH Ground Control, version {}",
        crate_version!()
    )]
}

macro_rules! command_help {
    ($cmd:expr, $help:expr) => {
        format!("{}: {}", $cmd, $help)
    };
}

/// Generate the help response for a help command
fn help_response() -> Vec<String> {
    vec![
        command_help!(command::WINGS, "request X attacking wingmen"),
        command_help!(command::CALL_OFF, "remove any requested wingmen"),
        command_help!(command::VERSION, "program version"),
    ]
}

/// A user's command for ground control
///
/// Given the context provided in a command,
/// ground control generates a response.
#[derive(Debug)]
pub struct Command<'s> {
    /// The user's message
    ///
    /// This is what they literally typed into the
    /// global / whisper chat. We'll parse this
    /// to generate a response
    message: &'s str,
    /// The current user
    user: &'s str,
    /// The current wings assigned to the user
    ///
    /// This isn't maintained here, so we expect the caller
    /// to keep track of this state
    wings: u8,
}

impl<'s> Command<'s> {
    /// Generate a new command for ground control
    pub fn new(message: &'s str, user: &'s str, wings: u8) -> Self {
        Command {
            message,
            user,
            wings,
        }
    }
}

/// Possible reasons for a failed command
#[derive(Debug, PartialEq, Eq)]
pub enum BadCommand<'s> {
    /// Unkown command (wrapped in the variant)
    Unknown(&'s str),
    /// No wings assigned to this user
    NoWings(&'s str),
    /// Too many wings assigned to this user
    TooManyWings(&'s str, u8),
    /// Wings are already assigned to this user
    AlreadyWinged(&'s str, u8),
}

impl<'s> fmt::Display for BadCommand<'s> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BadCommand::Unknown(cmd) => write!(f, "unknown command: '{}'", cmd),
            BadCommand::NoWings(user) => write!(f, "no wings assigned to {}", user),
            BadCommand::TooManyWings(user, max) => {
                write!(f, "too many wings attacking {} (max {} wings)", user, max)
            }
            BadCommand::AlreadyWinged(user, wings) => write!(
                f,
                "{} already has {} wings; use {} to remove",
                user,
                wings,
                command::CALL_OFF
            ),
        }
    }
}

/// A response generated for a valid command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseKind {
    /// Set wings for the specified user
    SetWings { wings: u8 },
    /// Remove all wings on the specified user
    ClearWings,
}

/// A ground control response
#[derive(Debug)]
pub struct Response {
    /// The message to send back to the user
    ///
    /// These are split up into multiple messages to
    /// circumvent any max character limit per message.
    /// We don't actually count characters now, but this
    /// is for any future need.
    message: Vec<String>,
    /// The kind of action to take on the maintained state
    kind: Option<ResponseKind>,
}

impl Response {
    /// Returns the message that should be relayed back to the user
    ///
    /// This destroys the response object to obtain the message. The
    /// message is a collection of strings that should be sent in
    /// sequence. Sent multiple messages so that we do not overload
    /// a single message size.
    pub fn msg(self) -> Vec<String> {
        self.message
    }

    /// Returns the kind of response and subsequent action to take
    pub fn kind(&self) -> Option<ResponseKind> {
        self.kind
    }

    /// Create a response containing just a message
    fn just_message(message: Vec<String>) -> Self {
        Response {
            message,
            kind: None,
        }
    }

    /// Create an 'add wings' response with a canned response message
    fn add_wings(user: &str, wings: u8) -> Self {
        Response {
            message: vec![format!("OK {}, {} wings are coming!", user, wings)],
            kind: Some(ResponseKind::SetWings { wings }),
        }
    }

    /// Create a 'clear wings' response with a canned response message
    fn clear_wings(user: &str) -> Self {
        Response {
            message: vec![format!("Calling off all wings from {}", user)],
            kind: Some(ResponseKind::ClearWings),
        }
    }
}

/// A control tower handles user commands and dispatches wings
pub struct ControlTower {
    /// The maximum number of wings allowed per user
    max_wings: u8,
}

impl ControlTower {
    /// Create a control tower that will limit the number of wings
    /// to the provided max
    pub fn new(max_wings: u8) -> Self {
        ControlTower { max_wings }
    }

    /// Command parsing implementation
    ///
    /// If we're in here, we know that the user's message represents some kind of
    /// command; it's not just a random message to another user.
    #[inline]
    fn parse_command_impl<'s>(&self, cmd: Command<'s>) -> Result<Response, BadCommand<'s>> {
        if cmd.message == command::HELP {
            Ok(Response::just_message(help_response()))
        } else if cmd.message == command::VERSION {
            Ok(Response::just_message(version_message()))
        } else if cmd.message.starts_with(command::WINGS) {
            if cmd.wings > 0 {
                Err(BadCommand::AlreadyWinged(cmd.user, cmd.wings))
            } else {
                // User may have requested wings
                let mut words = cmd.message.split_whitespace();
                words.next(); // --gc-wings
                match words.next().and_then(|count| count.parse().ok()) {
                    None => Err(BadCommand::Unknown(cmd.message)),
                    Some(count) if count > self.max_wings => {
                        Err(BadCommand::TooManyWings(cmd.user, self.max_wings))
                    }
                    Some(count) if count == 0 => Err(BadCommand::Unknown(cmd.message)),
                    Some(count) => Ok(Response::add_wings(cmd.user, count)),
                }
            }
        } else if cmd.message == command::CALL_OFF {
            if cmd.wings > 0 {
                Ok(Response::clear_wings(cmd.user))
            } else {
                Err(BadCommand::NoWings(cmd.user))
            }
        } else {
            Err(BadCommand::Unknown(cmd.message))
        }
    }

    /// Convert a user command into a response and response action
    ///
    /// Returns `None` if the user was not sending a message to ground control. If it
    /// seems like a message was intended from ground control, `Some(result)` is returned.
    /// A `BadCommand` is returned if the command is not understood by ground control. A
    /// `Response`, possibly with a response action, is returned on an appropriate command.
    pub fn parse_command<'s>(&self, cmd: Command<'s>) -> Option<Result<Response, BadCommand<'s>>> {
        if !cmd.message.starts_with(command::PREFIX) {
            // Not intended for ground control
            None
        } else {
            Some(self.parse_command_impl(cmd))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::BadCommand;
    use super::Command;
    use super::ControlTower;
    use super::ResponseKind;

    #[test]
    fn not_a_command() {
        let cmd = Command::new("--game-stats", "derps", 3);
        let ctrl = ControlTower::new(5);
        assert!(ctrl.parse_command(cmd).is_none());
    }

    #[test]
    fn request_help() {
        let cmd = Command::new("--gc-help", "putin copter", 0);
        let ctrl = ControlTower::new(5);
        let resp = ctrl.parse_command(cmd).unwrap();
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert!(resp.kind.is_none());
    }

    #[test]
    fn request_wings() {
        let cmd = Command::new("--gc-wings 3", "xplay", 0);
        let ctrl = ControlTower::new(5);
        let resp = ctrl
            .parse_command(cmd)
            .expect("parsed something")
            .expect("valid command");
        assert_eq!(
            resp.kind.expect("a response kind"),
            ResponseKind::SetWings { wings: 3 }
        )
    }

    #[test]
    fn request_wings_too_many() {
        let cmd = Command::new("--gc-wings 25", "STEAMROLLER", 0);
        let ctrl = ControlTower::new(5);
        let resp = ctrl
            .parse_command(cmd)
            .expect("parsed something")
            .expect_err("invalid command");
        assert_eq!(resp, BadCommand::TooManyWings("STEAMROLLER", 5));
    }

    #[test]
    fn request_wings_nan() {
        let cmd = Command::new("--gc-wings abc", "Detect", 0);
        let ctrl = ControlTower::new(5);
        let resp = ctrl
            .parse_command(cmd)
            .expect("parsed something")
            .expect_err("invalid command");
        assert_eq!(resp, BadCommand::Unknown("--gc-wings abc"));
    }

    #[test]
    fn request_wings_zero() {
        let cmd = Command::new("--gc-wings 0", "putin copter", 0);
        let ctrl = ControlTower::new(5);
        let resp = ctrl
            .parse_command(cmd)
            .expect("parsed something")
            .expect_err("invalid command");
        assert_eq!(resp, BadCommand::Unknown("--gc-wings 0"));
    }

    #[test]
    fn call_off() {
        let cmd = Command::new("--gc-call-off", "Friendo", 4);
        let ctrl = ControlTower::new(5);
        let resp = ctrl
            .parse_command(cmd)
            .expect("parsed something")
            .expect("valid command");
        assert_eq!(resp.kind.unwrap(), ResponseKind::ClearWings);
    }

    #[test]
    fn call_off_no_wings() {
        let cmd = Command::new("--gc-call-off", "xyz", 0);
        let ctrl = ControlTower::new(5);
        let resp = ctrl
            .parse_command(cmd)
            .expect("parsed something")
            .expect_err("invalid command");
        assert_eq!(resp, BadCommand::NoWings("xyz"));
    }
}
