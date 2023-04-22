// SPDX-FileCopyrightText: 2023 Brian Kubisiak <brian@kubisiak.com>
//
// SPDX-License-Identifier: GPL-3.0-only

use std::process::ExitCode;

use library::Library;
use magpie::CrdtPack;

fn main() -> ExitCode {
    env_logger::init();

    if let Err(e) = Library::init() {
        log::error!("{}", e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
