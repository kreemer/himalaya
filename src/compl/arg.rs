//! Module related to completion arguments.
//!
//! This module provides subcommands and an argument matcher related to completion.

use anyhow::Result;
use clap::{self, App, Arg, ArgMatches, Shell, SubCommand};
use log::debug;

/// Enumeration of all possible matches.
pub enum Match<'a> {
    /// Generate completion script for the given shell slice.
    Generate(&'a str),
}

/// Completion arg matcher.
pub fn matches<'a>(m: &'a ArgMatches) -> Result<Option<Match<'a>>> {
    if let Some(m) = m.subcommand_matches("completion") {
        debug!("completion command matched");
        let shell = m.value_of("shell").unwrap();
        debug!("shell: {}", shell);
        return Ok(Some(Match::Generate(shell)));
    };

    Ok(None)
}

/// Completion subcommands.
pub fn subcmds<'a>() -> Vec<App<'a, 'a>> {
    vec![SubCommand::with_name("completion")
        .about("Generates the completion script for the given shell")
        .args(&[Arg::with_name("shell")
            .possible_values(&Shell::variants()[..])
            .required(true)])]
}