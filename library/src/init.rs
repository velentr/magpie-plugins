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
